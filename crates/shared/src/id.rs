use std::{fmt::Debug, marker::PhantomData};

#[derive(Clone, Default)]
pub struct IdVec<I: Id, T> {
    inner: Vec<T>,
    _marker: PhantomData<I>,
}

impl<I: Id, T> IdVec<I, T> {
    pub fn new() -> Self {
        Self {
            inner: vec![],
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, t: T) -> I {
        let id = I::new(self.inner.len() as u32);
        self.inner.push(t);
        id
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn get(&self, id: I) -> Option<&T> {
        self.inner.get(id.index())
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        self.inner.get_mut(id.index())
    }

    pub fn values(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn values_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn into_values(self) -> std::vec::IntoIter<T> {
        self.inner.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.inner
            .iter()
            .enumerate()
            .map(|(idx, value)| (I::new(idx as u32), value))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> {
        self.inner
            .iter_mut()
            .enumerate()
            .map(|(idx, value)| (I::new(idx as u32), value))
    }
}

impl<I: Id, T> IntoIterator for IdVec<I, T> {
    type Item = (I, T);

    type IntoIter =
        std::iter::Map<std::iter::Enumerate<std::vec::IntoIter<T>>, fn((usize, T)) -> (I, T)>;

    fn into_iter(self) -> Self::IntoIter {
        fn map<I: Id, T>((idx, value): (usize, T)) -> (I, T) {
            (I::new(idx as u32), value)
        }

        self.inner.into_iter().enumerate().map(map::<I, T>)
    }
}

impl<'a, I: Id, T> IntoIterator for &'a IdVec<I, T> {
    type Item = (I, &'a T);

    type IntoIter = std::iter::Map<
        std::iter::Enumerate<std::slice::Iter<'a, T>>,
        fn((usize, &'a T)) -> (I, &'a T),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn map<I: Id, T>((idx, value): (usize, &T)) -> (I, &T) {
            (I::new(idx as u32), value)
        }

        self.inner.iter().enumerate().map(map::<I, T>)
    }
}

impl<'a, I: Id, T> IntoIterator for &'a mut IdVec<I, T> {
    type Item = (I, &'a mut T);

    type IntoIter = std::iter::Map<
        std::iter::Enumerate<std::slice::IterMut<'a, T>>,
        fn((usize, &'a mut T)) -> (I, &'a mut T),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn map<I: Id, T>((idx, value): (usize, &mut T)) -> (I, &mut T) {
            (I::new(idx as u32), value)
        }

        self.inner.iter_mut().enumerate().map(map::<I, T>)
    }
}

impl<I: Id, T> std::ops::Index<I> for IdVec<I, T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index.index()]
    }
}

impl<I: Id, T> std::ops::IndexMut<I> for IdVec<I, T> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index.index()]
    }
}

impl<I: Id, T> std::ops::Index<&I> for IdVec<I, T> {
    type Output = T;

    fn index(&self, index: &I) -> &Self::Output {
        &self.inner[index.index()]
    }
}

impl<I: Id, T> std::ops::IndexMut<&I> for IdVec<I, T> {
    fn index_mut(&mut self, index: &I) -> &mut Self::Output {
        &mut self.inner[index.index()]
    }
}

impl<I: Id, T> From<Vec<T>> for IdVec<I, T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            inner: value,
            _marker: PhantomData,
        }
    }
}

impl<I: Id, T: Debug> std::fmt::Debug for IdVec<I, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.inner.iter()).finish()
    }
}

pub trait Id: From<u32> + Into<u32> + Copy {
    fn new(idx: u32) -> Self;

    #[inline(always)]
    fn index(self) -> usize {
        let idx: u32 = self.into();
        idx as usize
    }
}

#[macro_export]
macro_rules! id {
    ($($vis:vis $name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
            $vis struct $name(u32);

            impl $name {
                pub const ZERO: Self = Self(0);

                /// Sentinel for "no id here" -- safe to read as a discriminator, unsafe to index with.
                pub const DANGLING: Self = Self(u32::MAX);

                pub fn index(self) -> usize {
                    self.0 as usize
                }
            }

            impl $crate::Id for $name {
                fn new(idx: u32) -> Self {
                    Self(idx)
                }
            }

            impl From<$name> for u32 {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            impl From<u32> for $name {
                fn from(value: u32) -> Self {
                    Self(value)
                }
            }
        )*
    }
}
