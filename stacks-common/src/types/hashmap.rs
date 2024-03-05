use std::hash::Hash;
use std::iter::{FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "testing")]
use fake::{Dummy, Fake, Faker};
use hashbrown::HashMap;
use rand::Rng;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct StacksHashMap<K, V>(pub HashMap<K, V>)
where
    K: Eq + Hash;

impl<K, V> StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        StacksHashMap(HashMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        StacksHashMap(HashMap::with_capacity(capacity))
    }
}

impl Default for StacksHashMap<String, String> {
    fn default() -> Self {
        StacksHashMap(HashMap::new())
    }
}

impl<'a, K, V> Deref for StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> DerefMut for StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V> FromIterator<(K, V)> for StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = StacksHashMap::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

#[cfg(feature = "testing")]
impl<K, V> Dummy<Faker> for StacksHashMap<K, V>
where
    K: Eq + Hash + Dummy<Faker>,
    V: Dummy<Faker>,
{
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> Self {
        let mut map = HashMap::<K, V>::new();
        for _ in 0..rng.gen_range(1..5) {
            map.insert(Faker.fake(), Faker.fake());
        }
        StacksHashMap(map)
    }
}