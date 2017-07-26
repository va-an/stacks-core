import os, tempfile, sys, time
import json, logging
import sqlite3
import thread, threading
import BaseHTTPServer, requests
import jsonschema

from blockstack_client import schemas, subdomains
from blockstack_client import constants as blockstack_constants
import blockstack_zones

from . import config

log = logging.getLogger(__name__)
log.setLevel(logging.DEBUG)
fh = logging.FileHandler(config.get_logfile())
fh.setLevel(logging.DEBUG)
#fh.setFormatter(formatter)
log.addHandler(fh)

class SubdomainOpsQueue(object):
    def __init__(self, domain, db_path, entries_per_tx = 100):
        dirname = os.path.dirname(db_path)
        if not os.path.exists(dirname):
            os.makedirs(dirname)
        self.domain = domain
        self.conn = sqlite3.connect(db_path)
        self.queue_table = "queue_{}".format(domain.replace('.', '_'))
        self._create_if_needed()
        self.entries_per_tx = entries_per_tx

    def _create_if_needed(self):
        queue = """CREATE TABLE {} (
        received_at INTEGER PRIMARY KEY,
        subdomain TEXT NOT NULL,
        subdomain_name TEXT NOT NULL,
        in_tx TEXT);
        """.format(self.queue_table)

        index = "CREATE INDEX subdomain_{} ON {} (subdomain)".format(
            self.queue_table, self.queue_table)

        check = """SELECT name FROM sqlite_master WHERE
         type='table' AND name='{}';""".format(self.queue_table)

        c = self.conn.cursor()
        r_val = self._execute(check, ()).fetchall()
        if len(r_val) > 0:
            return
        c.execute(queue)
        c.execute(index)
        self.conn.commit()

    def _execute(self, sql, args):
        c = self.conn.cursor()
        r_val = c.execute(sql, args)
        self.conn.commit()
        return r_val

    def _add_subdomain_row(self, jsoned_strings, subdomain_name):
        sql = """INSERT INTO {} (subdomain, subdomain_name)
                 SELECT ?, ?
                 WHERE NOT EXISTS ( SELECT 1 FROM {} WHERE subdomain_name = ? )""".format(
                     self.queue_table, self.queue_table)
        inserted = self._execute(sql, (jsoned_strings, subdomain_name, subdomain_name)).rowcount
        if inserted <= 0:
            raise subdomains.SubdomainAlreadyExists(subdomain_name, self.domain)

    def _get_queued_rows(self):
        sql = """SELECT received_at, subdomain FROM {} 
        WHERE in_tx ISNULL ORDER BY received_at ASC LIMIT {};
        """.format(self.queue_table, config.get_tx_limit())
        out = list(self._execute(sql, ()).fetchall())
        return [ (received_at, 
                  subdomains.Subdomain.parse_subdomain_record(json.loads(packed_subdomain))) 
                 for received_at, packed_subdomain in out ]

    def _set_in_tx(self, subds, txid):
        sql = """UPDATE {} SET in_tx = ?
        WHERE received_at IN ({})""".format(
            self.queue_table,
            ",".join("?" * len(subds)))
        self._execute(sql, [txid] + list(subds))

    def add_subdomain_to_queue(self, subdomain):
        name = subdomain.name
        packed_dict = subdomain.as_zonefile_entry()
        jsoned = json.dumps(packed_dict)
        self._add_subdomain_row(jsoned, name)

    def submit_transaction(self):
        queued_rows = self._get_queued_rows()
        if len(queued_rows) == 0:
            return {'status' : 'true',
                    'subdomain_updates' : 0}
        indexes, entries = zip(* queued_rows)

        zonefile_made = False
        to_add = list(entries)

        zf_txt, subs_failed = subdomains.add_subdomains(
            to_add, self.domain, broadcast_tx = False)

        if len(subs_failed) > 0:
            indexes = list(indexes)
            db_indexes_failed = []
            subs_failed.sort(reverse=True)
            for i in subs_failed:
                db_indexes_failed.append(indexes.pop(i))
            log.info("Subdomain already existed for ({})".format(
                [ entries[i].name for i in subs_failed ] ))
            self._set_in_tx(db_indexes_failed, "ALREADYEXISTED")
            if len(indexes) == 0:
                return {'status' : 'true',
                        'subdomain_updates' : 0}

        # issue user zonefile update to API endpoint

        target = "/v1/names/{}/zonefile".format(self.domain)
        resp = rest_to_api(target, data = json.dumps({'zonefile' : zf_txt}), call = requests.put)

        if resp.status_code != 202:
            msg = 'Error submitting subdomain bundle: {}'.format(resp.text)
            log.error(msg)
            try:
                resp_js = resp.json()
                if "maxLength" in str(resp_js["error"]):
                    self.entries_per_tx = 0.8 * int(self.entries_per_tx)
                    
            except Exception as e:
                pass

            self._set_in_tx(indexes, msg)
            return {'error' : msg}

        try:
            resp_js = resp.json()
        except Exception as e:
            log.error("Error in response: {}".format(resp))
            log.exception(e)
            return {'error' : 'Error in parsing response'}

        if 'error' in resp_js:
            msg = 'Error submitting subdomain bundle: {}'.format(tx_resp['error'])
            log.error(msg)
            self._set_in_tx(indexes, msg)
            return {'error' : msg}

        txid = str(resp_js['transaction_hash'])
        self._set_in_tx(indexes, txid)

        log.info('Issued update for {} subdomain entries. In tx: {}'.format(
            len(indexes), txid))
        return {'status' : 'true',
                'subdomain_updates' : len(indexes),
                'transaction_hash' : txid}

def queue_name_for_registration(subdomain, domain_name):
    try:
        subdomains.resolve_subdomain(subdomain.name, domain_name)
        raise subdomains.SubdomainAlreadyExists(subdomain.name, domain_name)
    except subdomains.SubdomainNotFound as e:
        pass
    q =  SubdomainOpsQueue(domain_name, config.get_subdomain_registrar_db_path())
    q.add_subdomain_to_queue(subdomain)
    return {'status' : 'true',
            'message' : 'Subdomain registration queued.'}

def parse_subdomain_request(input_str):
    schema = {
        'type' : 'object',
        'properties' : {
            'subdomain' : {
                'type': 'string',
                'pattern': r'([a-z0-9\-_+]{{{},{}}})$'.format(3, 36)
            },
            'data_pubkey' : {
                'type': 'string',
                'pattern': r'^(pubkey:data:[0-9a-fA-F]+)$'
            },
            'uris' : {
                'type': 'array',
                'items': schemas.URI_RECORD_SCHEMA
            },
            'zonefile_str' : {
                'type' : 'string',
                'maxLength' : blockstack_constants.RPC_MAX_ZONEFILE_LEN
            }
        }
    }
    request = json.loads(input_str)
    jsonschema.validate(request, schema)
    
    zonefile_str = None
    if 'zonefile_str' in request:
        zonefile_str = request['zonefile_str']
    elif 'uris' in request:
        zonefile_dict = {
            '$origin' : request['subdomain'],
            '$ttl' : 3600,
            'uri' : request['uris']
        }
        zonefile_str = blockstack_zones.make_zone_file(zonefile_dict)
    if zonefile_str is None:
        raise Exception("Request lacked either a zonefile_str or an uris entry")

    pubkey_entry = subdomains.encode_pubkey_entry(
        subdomains.decode_pubkey_entry(str(request['data_pubkey'])))

    return subdomains.Subdomain(
        request['subdomain'], pubkey_entry,
        n=0, zonefile_str = zonefile_str)

def set_registrar_state(domain_name):
    """
    Set singleton state and start the registrar thread.
    Return the registrar state on success
    Return None on error
    """
    lockfile = config.get_lockfile()

    # if we're already running, then bail
    if SubdomainLock.is_lockfile_valid( lockfile ):
        log.debug("SubdomainRegistrarWorker already initialized")
        return None

    try:
        SubdomainLock.acquire( lockfile )
    except (IOError, OSError):
        try:
            os.unlink(path)
        except:
            pass

        log.debug("Extra worker exiting (failed to lock)")
        return

    log.info("Initialize Subdomain Registrar State from %s" % (lockfile))

    registrar_thread = SubdomainRegistrarWorker(domain_name)
    registrar_thread.start()

    api_thread = SubdomainRegistrarRPCWorker(domain_name)
    api_thread.start()

    while True:
        try:
            time.sleep(60)
        except (KeyboardInterrupt, Exception) as e:
            log.info("Interrupt received, beginning shutdown.")
            api_thread.request_stop()
            api_thread.join()
            registrar_thread.request_stop()
            registrar_thread.join()
            log.info("Registrar threads shutdown")
            return

class SubdomainRegistrarWorker(threading.Thread):
    def __init__(self, domain_name):
        super(SubdomainRegistrarWorker, self).__init__()
        self.domain_name = domain_name
        db_path = config.get_subdomain_registrar_db_path()
        self.queue_opts = (domain_name, db_path)
        self.running = True
        self.tx_every = config.get_tx_frequency()
    def request_stop(self):
        self.running = False
    def run(self):
        queue = SubdomainOpsQueue(*self.queue_opts)
        while self.running:
            # todo: wake up more frequently, poll blocks,
            #        track last block with tx, and do tx_every in
            #        block time, rather than clock time.
            queue.submit_transaction()

            for i in xrange(0, int(self.tx_every)):
                try:
                    time.sleep(1)
                except:
                    log.debug("Subdomain's sleep interrupted")
                if not self.running:
                    return

class SubdomainRegistrarRPCWorker(threading.Thread):
    def __init__(self, domain_name):
        super(SubdomainRegistrarRPCWorker, self).__init__()
        bind_address = config.get_api_bind_address()
        bind_port = config.get_api_bind_port()
        self.server = BaseHTTPServer.HTTPServer(
            (bind_address, bind_port), SubdomainRegistrarRPCHandler)
        self.server.domain_name = domain_name
    def request_stop(self):
        self.server.shutdown()
    def run(self):
        self.server.serve_forever()

class SubdomainRegistrarRPCHandler(BaseHTTPServer.BaseHTTPRequestHandler):
    def do_POST(self):
        self.send_header("Content-Type", "application/json") 
        if str(self.path) != "/register":
            return self.send_response(404, json.dumps({"error" : "Unsupported API method"}))
        length = int(self.headers.getheader('content-length'))
        if length > 1024 * 1024:
            return self.send_response(403, json.dumps({"error" : "Content length too long. Request Denied."}))
        try:
            subdomain = parse_subdomain_request(self.rfile.read(length))
        except Exception as e:
            log.exception(e)
            return self.send_response(401, json.dumps({"error" : "Problem parsing request"}))

        try:
            queued_resp = queue_name_for_registration(subdomain, self.server.domain_name)
        except subdomains.SubdomainAlreadyExists as e:
            log.exception(e)
            return self.send_response(403, json.dumps({"error" : "Subdomain already exists on this domain"}))

        if "error" in queued_resp:
            return self.send_response(500, json.dumps(queued_resp))
        return self.send_response(202, json.dumps(queued_resp))

class SubdomainLock(object):
    @staticmethod
    def is_lockfile_stale(path):
        with open(path, "r") as f:
            dat = f.read()
            try:
                pid = int(dat.strip())
            except:
                # corrupt
                pid = -1
        return pid != os.getpid()
    @staticmethod
    def lockfile_write( fd ):        
        buf = "%s\n" % os.getpid()
        nw = 0
        while nw < len(buf):
            try:
                rc = os.write( fd, buf[nw:] )
                nw += rc
            except:
                log.error("Failed to write lockfile")
                return False
        return True
    @staticmethod
    def acquire( lockfile_path ):
        dirname = os.path.dirname(lockfile_path)
        if not os.path.exists(dirname):
            os.makedirs(dirname)

        fd, path = tempfile.mkstemp(prefix=".subd.registrar.lock.", dir=dirname)
        os.link( path, lockfile_path )
        try:
            os.unlink(path)
        except:
            pass

        # success!  write the lockfile
        rc = SubdomainLock.lockfile_write( fd )
        os.close( fd )

        if not rc:
            log.error("Failed to write lockfile")
            raise IOError("Failed to write lockfile")

    @staticmethod
    def is_lockfile_valid( lockfile_path ):
        """
        Does the lockfile exist and does it correspond
        to a running registrar?
        """
        if os.path.exists(lockfile_path):
            # is it stale?
            if SubdomainLock.is_lockfile_stale( lockfile_path ):
                log.debug("Removing stale subdomain lockfile")
                os.unlink(lockfile_path)
                return False
            else:
                # not stale
                return True
        else:
            return False


def rest_to_api(target, data=None, call = requests.get):
    api_endpoint, authentication = config.get_core_api_endpoint()
    headers = {}
    headers['authorization'] = 'bearer {}'.format(authentication)
    headers['origin'] = 'http://localhost:3000' # lies.
    headers['content-type'] = 'application/json'
    target = api_endpoint + target

    if data is None:
        return call(target, headers = headers)
    else:
        return call(target, headers = headers,
                    data = data)

START_HELP = """ usage: service start <domainname:required> """

if __name__ == "__main__":
    command = sys.argv.pop(1)
    if command == "start":
        if len(sys.argv) < 2:
            print START_HELP
            exit(1)
        domain_name = sys.argv[1]
        set_registrar_state(domain_name)
    elif command == "stop":
        print "lol. whatever."
