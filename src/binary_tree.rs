use std::{cmp::Ordering, iter, ptr::NonNull};

type RawBinTreePtr<T> = Option<NonNull<RawBinTree<T>>>;

trait NodeStore<T: ?Sized>: Default {
    type Node;

    #[inline]
    fn insert(&mut self, value: T) -> Self::Node
    where
        T: Sized,
    {
        self.insert_boxed(Box::new(value))
    }

    /// Transfers ownership of `node` to this store
    ///
    /// All safety conditions of `Box::from_raw()` must hold for `node`
    #[inline]
    unsafe fn insert_from_ptr(&mut self, value: *mut T) -> Self::Node {
        unsafe { self.insert_boxed(Box::from_raw(value)) }
    }

    fn insert_boxed(&mut self, value: Box<T>) -> Self::Node;

    #[inline]
    fn delete(&mut self, node: Self::Node) -> T
    where
        T: Sized,
    {
        *self.delete_boxed(node)
    }

    fn delete_boxed(&mut self, node: Self::Node) -> Box<T>;

    fn get(&self, node: &Self::Node) -> &T;

    fn get_mut(&mut self, node: &Self::Node) -> &mut T;
}

#[derive(Debug, Default, Clone, Copy)]
struct HeapNodeStore;

impl<T: ?Sized> NodeStore<T> for HeapNodeStore {
    type Node = HeapNode<T>;

    #[inline(always)]
    fn insert_boxed(&mut self, value: Box<T>) -> Self::Node {
        HeapNode(NonNull::new(Box::into_raw(value)).unwrap())
    }

    #[inline(always)]
    fn delete_boxed(&mut self, node: Self::Node) -> Box<T> {
        unsafe { Box::from_raw(node.0.as_ptr()) }
    }

    #[inline(always)]
    fn get(&self, node: &Self::Node) -> &T {
        unsafe { node.0.as_ref() }
    }

    #[inline(always)]
    fn get_mut(&mut self, node: &Self::Node) -> &mut T {
        unsafe { &mut *node.0.as_ptr() }
    }
}

/// Not clone/copy because we're not keeping track of deletion so a double-free can be called
#[derive(Debug, PartialEq, Eq)]
struct HeapNode<T: ?Sized>(NonNull<T>);

// struct

pub struct RawBinTree<T: ?Sized> {
    parent: RawBinTreePtr<T>,
    children: [RawBinTreePtr<T>; 2],
    data: T,
}

impl<T: ?Sized> RawBinTree<T> {
    pub fn new_alloc_ptr(data: T) -> NonNull<Self>
    where
        T: Sized,
    {
        NonNull::from_mut(Self::new_alloc(data))
    }

    pub fn new_alloc<'a>(data: T) -> &'a mut Self
    where
        T: Sized,
    {
        Box::leak(Box::new(Self {
            parent: None,
            children: [None; 2],
            data,
        }))
    }

    pub fn children(&self) -> [RawBinTreePtr<T>; 2] {
        self.children
    }

    pub fn left(&self) -> RawBinTreePtr<T> {
        self.children[0]
    }

    pub fn right(&self) -> RawBinTreePtr<T> {
        self.children[1]
    }

    // pub fn set_left(&mut self, left: RawBinTreePtr<T>) {
    //     self.children[0] = left;
    // }

    // pub fn set_right(&mut self, right: RawBinTreePtr<T>) {
    //     self.children[1] = right;
    // }

    pub fn set_child(&mut self, node: Option<&mut Self>, child: usize) {
        match node {
            Some(node) => {
                node.parent = Some(NonNull::from_mut(self));
                self.children[child] = Some(NonNull::from_mut(node));
            }
            None => self.children[child] = None,
        }
    }

    pub unsafe fn traverse_preorder(&self) -> impl Iterator<Item = &Self> {
        // self, left, right
        iter::from_fn(move || None)
    }

    pub fn traverse_inorder(&self) -> impl Iterator<Item = &Self> {
        // left, self, right
        iter::from_fn(move || None)
    }

    pub fn traverse_postorder(&self) -> impl Iterator<Item = &Self> {
        // left, right, self
        iter::from_fn(move || None)
    }
}

/// Finds the smallest node in a given BST
unsafe fn smallest<T>(bst: NonNull<RawBinTree<T>>) -> NonNull<RawBinTree<T>> {
    unsafe { follow_child(bst, 0) }
}

/// Finds the smallest node in a given BST
unsafe fn largest<T>(bst: NonNull<RawBinTree<T>>) -> NonNull<RawBinTree<T>> {
    unsafe { follow_child(bst, 1) }
}

unsafe fn follow_child<T>(mut bst: NonNull<RawBinTree<T>>, child: usize) -> NonNull<RawBinTree<T>> {
    while let Some(next) = unsafe { (*bst.as_ptr()).children[child] } {
        bst = next;
    }

    bst
}

pub struct BasicBST<T: Ord> {
    head: RawBinTreePtr<T>,
}

impl<T: Ord> BasicBST<T> {
    pub fn insert(&mut self, value: T) {
        let Some(mut head) = self.head else {
            self.head = Some(RawBinTree::new_alloc_ptr(value));
            return;
        };

        let mut head = unsafe { head.as_mut() };
        loop {
            let insert_to = match value.cmp(&head.data) {
                Ordering::Less => 0,
                Ordering::Equal => return,
                Ordering::Greater => 1,
            };

            match head.children[insert_to] {
                Some(mut child) => head = unsafe { child.as_mut() },
                None => {
                    head.set_child(Some(RawBinTree::new_alloc(value)), insert_to);

                    return;
                }
            }
        }
    }

    pub fn remove(&mut self, value: &T) {
        fn remove_inner<T: Ord>(head: RawBinTreePtr<T>, value: &T) -> RawBinTreePtr<T> {
            let head = unsafe { head?.as_mut() };

            match value.cmp(&head.data) {
                Ordering::Less => remove_inner(head.children[0], value),
                Ordering::Equal => {
                    let returned_ptr = match head.children {
                        [None, None] => None,
                        // If there is 1 child, the new root is this child
                        [None, p] | [p, None] => p,
                        // If there are 2 children, it's complicated
                        [Some(l), Some(r)] => {
                            // Swap with successor and delete
                            let successor = unsafe { smallest(r).as_mut() };
                            std::mem::swap(&mut successor.data, &mut head.data);

                            todo!()
                        }
                    };

                    //

                    returned_ptr
                }
                Ordering::Greater => remove_inner(head.children[1], value),
            }
        }

        self.head = remove_inner(self.head, value);
    }
}
