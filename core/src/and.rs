/*!
The [`And`] type.
*/

/**
Two values combined by and-ing.

When combined values don't have boolean semantics of their own this type is like a join or concatenation.
*/
pub struct And<T, U> {
    left: T,
    right: U,
}

impl<T, U> And<T, U> {
    /**
    And two values together.
    */
    pub const fn new(left: T, right: U) -> Self {
        And { left, right }
    }

    /**
    Get a reference to the first, or left-hand side.
    */
    pub const fn left(&self) -> &T {
        &self.left
    }

    /**
    Get a mutable reference to the first, or left-hand side.
    */
    pub fn left_mut(&mut self) -> &mut T {
        &mut self.left
    }

    /**
    Get a reference to the second, or right-hand side.
    */
    pub const fn right(&self) -> &U {
        &self.right
    }

    /**
    Get a mutable reference to the second, or right-hand side.
    */
    pub fn right_mut(&mut self) -> &mut U {
        &mut self.right
    }

    /**
    Split the combined values.
    */
    pub fn into_inner(self) -> (T, U) {
        (self.left, self.right)
    }
}
