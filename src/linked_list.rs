use std::ptr::NonNull;

pub trait LinkedList<T> {
    type Id;

    fn new() -> Self;

    fn insert(&mut self, item: T, pos: Self::Id);

    fn push_front(&mut self, item: T) -> Self::Id;
    fn push_back(&mut self, item: T) -> Self::Id;

    fn pop_front(&mut self) -> Option<T>;
    fn pop_back(&mut self) -> Option<T>;

    fn get(&self, item: Self::Id) -> &T;
    fn get_mut(&mut self, item: Self::Id) -> &mut T;

    fn next(&self, item: Self::Id) -> Option<Self::Id>;
    fn prev(&self, item: Self::Id) -> Option<Self::Id>;
}

type PtrLLNodePtr<T> = Option<NonNull<PtrLLNode<T>>>;

pub struct PtrLLId<T>(PtrLLNodePtr<T>);

struct PtrLLNode<T> {
    next: PtrLLNodePtr<T>,
    data: T,
}

pub struct PtrLL<T> {
    start: PtrLLNodePtr<T>,
    end: PtrLLNodePtr<T>,
}

impl<T> LinkedList<T> for PtrLL<T> {
    type Id = PtrLLId<T>;

    fn new() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    fn insert(&mut self, item: T, pos: Self::Id) {
        todo!()
    }

    fn push_front(&mut self, item: T) -> Self::Id {
        todo!()
    }

    fn push_back(&mut self, item: T) -> Self::Id {
        todo!()
    }

    fn pop_front(&mut self) -> Option<T> {
        todo!()
    }

    fn pop_back(&mut self) -> Option<T> {
        todo!()
    }

    fn get(&self, item: Self::Id) -> &T {
        todo!()
    }

    fn get_mut(&mut self, item: Self::Id) -> &mut T {
        todo!()
    }

    fn next(&self, item: Self::Id) -> Option<Self::Id> {
        todo!()
    }

    fn prev(&self, item: Self::Id) -> Option<Self::Id> {
        todo!()
    }
}
