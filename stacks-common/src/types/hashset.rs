use std::hash::Hash;
use std::iter::{FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "testing")]
use fake::{Dummy, Fake, Faker};
use hashbrown::HashSet;
use rand::Rng;

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct StacksHashSet<T>(pub hashbrown::HashSet<T>)
where
    T: Eq + Hash;

impl<T> StacksHashSet<T>
where
    T: Eq + Hash,
{
    pub fn new() -> Self {
        StacksHashSet(hashbrown::HashSet::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        StacksHashSet(hashbrown::HashSet::with_capacity(capacity))
    }
}

impl<T> Deref for StacksHashSet<T>
where
    T: Eq + Hash,
{
    type Target = hashbrown::HashSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for StacksHashSet<T>
where
    T: Eq + Hash,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Iterator for StacksHashSet<T>
where
    T: Eq + Hash + Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.clone().into_iter().next()
    }
}

impl<T> FromIterator<T> for StacksHashSet<T>
where
    T: Eq + Hash,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = StacksHashSet(HashSet::new());
        for item in iter {
            set.insert(item);
        }
        set
    }
}

impl<T> From<HashSet<T>> for StacksHashSet<T>
where
    T: Eq + Hash,
{
    fn from(set: HashSet<T>) -> Self {
        StacksHashSet(set)
    }
}

impl<T> From<&HashSet<T>> for StacksHashSet<T>
where
    T: Eq + Hash + Clone,
{
    fn from(set: &HashSet<T>) -> Self {
        StacksHashSet(set.clone())
    }
}

impl<T> Into<HashSet<T>> for StacksHashSet<T>
where
    T: Eq + Hash,
{
    fn into(self) -> HashSet<T> {
        self.0
    }
}

#[cfg(feature = "testing")]
impl<T> Dummy<Faker> for StacksHashSet<T>
where
    T: Dummy<Faker> + Eq + Hash,
{
    fn dummy_with_rng<R: Rng + ?Sized>(config: &Faker, rng: &mut R) -> Self {
        let len = rng.gen_range(1..5);
        let mut set = StacksHashSet::new();
        for _ in 0..len {
            set.insert(T::dummy_with_rng(&config, rng));
        }
        set
    }
}
