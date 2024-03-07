use std::hash::Hash;
use std::iter::{FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut};

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

impl<K, V> Default for StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        StacksHashMap(HashMap::<K, V>::new())
    }
}

impl<K, V> From<HashMap<K, V>> for StacksHashMap<K, V>
where
    K: Eq + Hash,
{
    fn from(map: HashMap<K, V>) -> Self {
        StacksHashMap(map)
    }
}

impl<K, V> From<&HashMap<K, V>> for StacksHashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn from(map: &HashMap<K, V>) -> Self {
        StacksHashMap(map.clone())
    }
}

impl<K, V> Into<HashMap<K, V>> for StacksHashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn into(self) -> HashMap<K, V> {
        self.0
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
