//! `mine_data_structs` is a simple crate which contains the data structs needed
//! to interact with minecraft, curse and modrinth API
//!
//! It also contains quality of life functions and specific methods/getters
//! which may come handy.
//!
//! As you can see when looking the structures of this lib, I'm avoiding to use
//! `Vec<T>` type and instead use `Box[T]`. Why ? (You should be asking yourself
//! if you dont know why).
//!
//! **BECAUSE A REPONSE OF THE API MUST ME A READONLY DATA**
//!
//! Why do you want to add whatever to the response you just got ?
//! "It is how it is and it isnt how it isnt."
//!
//! Anyways, if you feel like adding elements the boxes you can convert them to
//! vecs by using:
//!
//! ```rust no_run
//! let boxed_slice: Box<[i32]> = Box::from([1,2,3,4,5,6,7,8,9,10]);
//! let v = Vec::from(boxed_slice);
//! ```
//!
//! See ?? with that code only 1 allocation occurs since the vector will take
//! ownership of the pointer. Yay !

pub mod curse;
pub mod maker;
pub mod minecraft;
pub mod rinth;
