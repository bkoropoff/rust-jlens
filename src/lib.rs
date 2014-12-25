//! Extract data from JSON
//!
//! This crate provides a simple domain-specific language based on
//! method chaining to construct and run queries against
//! `serialize::json::Json` objects.
//!
//! An object implementing the `Selector` trait describes how to
//! select a set of nodes starting at a given path in a JSON document.
//! The most basic selector can be created with the `node()` function:
//! this selector always selects precisely the path given to it.  All
//! selectors have methods such as `child()` and `key()` which return
//! a new selector.  The new selector will select nodes relative to
//! the output of the original according to some criteria.  For
//! example, `node().child()` selects all children of the initial
//! node, while `node().child().child()` selects all children of the
//! children of the initial node, and so on.  By continuing to chain
//! method calls in this manner, a selector object representing a
//! complex query expression can be built up.  Example:
//!
//! ```
//! # #![feature(globs)]
//! # extern crate serialize;
//! # extern crate jlens;
//! # use serialize::json;
//! # use jlens::*;
//! # fn main() {
//! // Test JSON document
//! let json = json::from_str(r#"
//! [
//!    {
//!        "foo": ["Hello, world!", 3.14, false]
//!    },
//!    {
//!        "foo": [42, true]
//!    },
//!    {
//!        "foo": "Nope"
//!    },
//!    {
//!        "bar": [42, "Hello, world!"]
//!    }
//! ]"#).unwrap();
//!
//! // Given a list, match all objects in it that
//! // have a "foo" key where the value is a list
//! // that contains either the string "Hello, world!"
//! // or the u64 42
//! let matches = json.query(
//!     list().child().wherein(
//!         key("foo").list().child().or(
//!             string().equals("Hello, world!"),
//!             uint64().equals(42))));
//!
//! // Expected matches
//! let match1 = json::from_str(
//!     r#"{"foo": ["Hello, world!", 3.14, false]}"#).unwrap();
//! let match2 = json::from_str(
//!     r#"{"foo": [42, true]}"#).unwrap();
//!
//! assert_eq!(matches.len(), 2);
//! assert!(matches.contains(& &match1));
//! assert!(matches.contains(& &match2));
//! # }
//! ```
//!
//! The `JsonExt` trait provides a convenience method on `Json`
//! objects which runs a selector and returns a `Vec<&Json>` of
//! results.

#![crate_type = "rlib"]
#![feature(unboxed_closures, globs)]

extern crate serialize;

use serialize::json::Json;
use std::collections::hash_set;

use JsonPath::{Root,Descendant};

/// JSON node path
///
/// Represents a path to a JSON node.
pub enum JsonPath<'a:'b,'b> {
    /// At the root node
    Root(&'a Json),
    /// At a node with the given parent path
    Descendant(&'a Json, &'b JsonPath<'a,'b>)
}

impl<'a,'b> Copy for JsonPath<'a,'b> {}

impl<'a,'b> JsonPath<'a,'b> {
    /// Create path at root node `r`
    #[inline]
    fn root(r: &'a Json) -> JsonPath<'a,'b> {
        Root(r)
    }

    /// Create descendant path of self at node `child`
    #[inline]
    fn descendant(&'b self, child: &'a Json) -> JsonPath<'a,'b> {
        Descendant(child, self)
    }

    /// Return the node this path points to
    #[inline]
    fn node(&self) -> &'a Json {
        match *self {
            Root(n) => n,
            Descendant(n, _) => n
        }
    }

    /// Return the parent path if this is not the root, otherwise `None`
    #[inline]
    fn parent(&self) -> Option<&'b JsonPath<'a,'b>> {
        match *self {
            Root(..) => None,
            Descendant(_, p) => Some(p)
        }
    }
}

/// JSON selector trait
///
/// Implementors of this trait select nodes from `Json` objects
/// according to some criteria.
pub trait Selector {
    /// Select matching nodes
    ///
    /// Given the path to a single node, `input`, this method should
    /// identify nodes to be selected and invoke the closure `f` with
    /// a path to each.
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>);

    /// Select current node if it is a `Json::Boolean`
    #[inline]
    fn boolean(self) -> BooleanSel<Self> {
        BooleanSel { inner: self }
    }

    /// Select current node if it is a `Json::U64`
    #[inline]
    fn uint64(self) -> U64Sel<Self> {
        U64Sel { inner: self }
    }

    /// Select current node if it is a `Json::I64`
    #[inline]
    fn int64(self) -> I64Sel<Self> {
        I64Sel { inner: self }
    }

    /// Select current node if it is a `Json::F64`
    #[inline]
    fn float64(self) -> F64Sel<Self> {
        F64Sel { inner: self }
    }

    /// Select current node if it is a `Json::String`
    #[inline]
    fn string(self) -> StringSel<Self> {
        StringSel { inner: self }
    }

    /// Select current node if it is a `Json::Object`
    #[inline]
    fn object(self) -> ObjectSel<Self> {
        ObjectSel { inner: self }
    }

    /// Select current node if it is a `Json::Array`
    #[inline]
    fn list(self) -> ListSel<Self> {
        ListSel { inner: self }
    }

    /// Select current node if it is a `Json::Null`
    #[inline]
    fn null(self) -> NullSel<Self> {
        NullSel { inner: self }
    }

    /// Select list element
    ///
    /// If the current node is a `Json::Array` of at least `index + 1`
    /// elements, selects the element at `index`.  Otherwise no nodes
    /// are selected.
    #[inline]
    fn at(self, index: uint) -> At<Self> {
        At { inner: self, index: index }
    }

    /// Select object value for key
    ///
    /// If the current node is a `Json::Object` that contains the key
    /// `name`, its value is selected.  Otherwise no nodes are
    /// selected.
    #[inline]
    fn key(self, name: &str) -> Key<Self> {
        Key { inner: self, name: name }
    }

    /// Select children of current node
    ///
    /// Selects all immediate child nodes of the current node: all
    /// elements of a `Json::Array`, or all values of a `Json::Object`.
    #[inline]
    fn child(self) -> Child<Self> {
        Child { inner: self }
    }

    /// Select parent of current node
    ///
    /// Selects the parent of the current node if it is not the root,
    /// otherwise no nodes are selected.
    #[inline]
    fn parent(self) -> Parent<Self> {
        Parent { inner: self }
    }

    /// Select descendents of current node
    ///
    /// Selects all child nodes of the current node and all their
    /// children, recursively.
    #[inline]
    fn descend(self) -> Descend<Self> {
        Descend { inner: self }
    }

    /// Select ancestors of current node
    ///
    /// Selects the parent, grandparent, etc. of the current node up
    /// to the root of the tree.
    #[inline]
    fn ascend(self) -> Ascend<Self> {
        Ascend { inner: self }
    }

    /// Select current node based on filter
    ///
    /// Runs the selector `filter` on the current node.  If it selects
    /// any nodes, the current node is selected.  If it does not select
    /// any nodes, no nodes are selected.
    #[inline]
    fn wherein<T:Selector>(self, filter: T) -> Wherein<Self,T> {
        Wherein { inner: self, filter: filter }
    }

    /// Select union of two selectors
    ///
    /// Runs `left` and `right` on the current node and selects
    /// nodes which are selected by either.
    #[inline]
    fn union<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Union<Self,T1,T2> {
        Union { inner: self, left: left, right: right }
    }

    /// Select intersection of two selectors
    ///
    /// Runs `left` and `right` on the current node and selects
    /// nodes which are selected by both.
    #[inline]
    fn intersect<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Intersect<Self,T1,T2> {
        Intersect { inner: self, left: left, right: right }
    }

    /// Select symmetric difference of two selectors
    ///
    /// Runs `left` and `right` on the current node, selecting
    /// nodes which are selected by `left` but not selected
    /// by `right`.
    ///
    /// Warning: this selector will execute its parent in the chain
    /// twice which may result in bad performance.
    #[inline]
    fn diff<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Diff<Self,T1,T2> {
        Diff { inner: self, left: left, right: right }
    }

    /// Select logical-and of two selectors
    ///
    /// Runs `left` and `right` on the current node and
    /// selects an arbitrary node if both selected at
    /// least one node themselves.  This is useful for
    /// encoding logical-and conditions for `wherein`.
    #[inline]
    fn and<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> AndSel<Self,T1,T2> {
        AndSel { inner: self, left: left, right: right }
    }

    /// Select logical-or of two selectors
    ///
    /// Runs `left` and `right` on the current node and
    /// selects an arbitrary node if either selected at
    /// least one node themselves.  This is useful for
    /// encoding logical-and conditions for `wherein`.
    #[inline]
    fn or<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> OrSel<Self,T1,T2> {
        OrSel { inner: self, left: left, right: right }
    }
}

pub struct Node {
    _dummy: ()
}

impl Copy for Node {}

impl Selector for Node {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        f(input)
    }
}

pub struct ObjectSel<S> {
    inner: S
}

impl<S:Selector> Selector for ObjectSel<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Object(..) => f(x),
                _ => ()
            }
        })
    }
}

pub struct ListSel<S> {
    inner: S
}

impl<S:Selector> Selector for ListSel<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Array(..) => f(x),
                _ => ()
            }
        })
    }
}

pub struct StringSel<S> {
    inner: S
}

pub struct StringEquals<'a,S> {
    inner: S,
    comp: &'a str
}

impl<S:Selector> StringSel<S> {
    /// Select current `Json::String` node if it is equal to `comp`
    #[inline]
    pub fn equals(self, comp: &str) -> StringEquals<S> {
        let StringSel { inner } = self;
        StringEquals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for StringSel<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::String(..) => f(x),
                _ => ()
            }
        })
    }
}

impl<'s,S:Selector> Selector for StringEquals<'s,S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::String(ref s) if self.comp == *s => f(x),
                _ => ()
            }
        })
    }
}

pub struct BooleanSel<S> {
    inner: S
}

pub struct BooleanEquals<S> {
    inner: S,
    comp: bool
}

impl<S:Selector> BooleanSel<S> {
    /// Select current `Json::Boolean` node if it is equal to `comp`
    #[inline]
    pub fn equals(self, comp: bool) -> BooleanEquals<S> {
        let BooleanSel { inner } = self;
        BooleanEquals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for BooleanSel<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Boolean(..) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for BooleanEquals<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Boolean(b) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct U64Sel<S> {
    inner: S
}

pub struct U64Equals<S> {
    inner: S,
    comp: u64
}

impl<S:Selector> U64Sel<S> {
    #[inline]
    pub fn equals(self, comp: u64) -> U64Equals<S> {
        let U64Sel { inner } = self;
        U64Equals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for U64Sel<S> {
    /// Select current `Json::U64` node if it is equal to `comp`
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::U64(..) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for U64Equals<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::U64(b) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct I64Sel<S> {
    inner: S
}

pub struct I64Equals<S> {
    inner: S,
    comp: i64
}

impl<S:Selector> I64Sel<S> {
    #[inline]
    pub fn equals(self, comp: i64) -> I64Equals<S> {
        let I64Sel { inner } = self;
        I64Equals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for I64Sel<S> {
    /// Select current `Json::I64` node if it is equal to `comp`
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::I64(..) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for I64Equals<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::I64(b) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct F64Sel<S> {
    inner: S
}

pub struct F64Equals<S> {
    inner: S,
    comp: f64
}

impl<S:Selector> F64Sel<S> {
    #[inline]
    pub fn equals(self, comp: f64) -> F64Equals<S> {
        let F64Sel { inner } = self;
        F64Equals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for F64Sel<S> {
    /// Select current `Json::F64` node if it is equal to `comp`
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::F64(..) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for F64Equals<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::F64(b) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct NullSel<S> {
    inner: S
}

impl<S:Selector> Selector for NullSel<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Null => f(x),
                _ => ()
            }
        })
    }
}

pub struct At<S> {
    inner: S,
    index: uint
}

impl<S:Selector> Selector for At<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Array(ref v) => {
                    if v.len() > self.index {
                        f(x.descendant(&v[self.index]))
                    }
                }
                _ => ()
            }
        })
    }
}

pub struct Key<'f,S> {
    inner: S,
    name: &'f str
}

impl<'f,S:Selector> Selector for Key<'f,S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Object(ref m) => {
                    match m.get(self.name) {
                        Some(e) => f(x.descendant(e)),
                        _ => ()
                    }
                },
                _ => ()
            }
        })
    }
}

pub struct Child<S> {
    inner: S
}

impl<S:Selector> Selector for Child<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            match x.node() {
                &Json::Object(ref m) => {
                    for (_,child) in m.iter() {
                        f(x.descendant(child))
                    }
                },
                &Json::Array(ref v) => {
                    for child in v.iter() {
                        f(x.descendant(child))
                    }
                },
                _ => ()
            }
        })
    }
}

pub struct Parent<S> {
    inner: S
}

impl<S:Selector> Selector for Parent<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen = hash_set::HashSet::new();
        self.inner.select(input, |x| {
            match x.parent() {
                Some(&p) => {
                    let j = p.node();
                    if !seen.contains(&(j as *const Json)) {
                        seen.insert(j as *const Json);
                        f(p)
                    }
                }
                _ => ()
            }
        })
    }
}

pub struct Descend<S> {
    inner: S
}

fn descend_helper<'a,'b,F>(input: JsonPath<'a,'b>,
                           seen: &mut hash_set::HashSet<*const Json>,
                           mut f: F)
                           where F: for<'c> FnMut(JsonPath<'a,'c>) {
    let j = input.node();
    if !seen.contains(&(j as *const Json)) {
        seen.insert(j as *const Json);
        match j {
            &Json::Object(ref m) => {
                for (_,c) in m.iter() {
                    let inner = input.descendant(c);
                    f(inner);
                    descend_helper(inner, seen, |x| f(x))
                }
            },
            &Json::Array(ref v) => {
                for c in v.iter() {
                    let inner = input.descendant(c);
                    f(inner);
                    descend_helper(inner, seen, |x| f(x))
                }
            },
            _ => ()
        }
    }
}

impl<S:Selector> Selector for Descend<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen = hash_set::HashSet::new();
        self.inner.select(input, |x| {
            descend_helper(x, &mut seen, |x| f(x))
        })
    }
}

pub struct Ascend<S> {
    inner: S
}

fn ascend_helper<'a,'b,F>(mut input: JsonPath<'a,'b>,
                          seen: &mut hash_set::HashSet<*const Json>,
                          mut f: F)
                          where F: for<'c> FnMut(JsonPath<'a,'c>) {
    loop {
        match input.parent() {
            Some(&x) => {
                let j = x.node();
                if !seen.contains(&(j as *const Json)) {
                    seen.insert(j as *const Json);
                    f(x);
                    input = x;
                } else {
                    break;
                }
            },
            _ => break
        }
    }
}

impl<S:Selector> Selector for Ascend<S> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen = hash_set::HashSet::new();
        self.inner.select(input, |n| {
            ascend_helper(n, &mut seen, |x| f(x));
        })
    }
}

pub struct Wherein<S,T> {
    inner: S,
    filter: T
}

impl<S:Selector,T:Selector> Selector for Wherein<S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        self.inner.select(input, |x| {
            let mut matches = false;
            self.filter.select(x, |_| matches = true);
            if matches {
                f(x)
            }
        })
    }
}

pub struct Union<I,S,T> {
    inner: I,
    left: S,
    right: T
}

impl<I:Selector,S:Selector,T:Selector> Selector for Union<I,S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen = hash_set::HashSet::new();
        self.inner.select(input, |x| {
            self.left.select(x, |x| {
                let j = x.node();
                if !seen.contains(&(j as *const Json)) {
                    seen.insert(j as *const Json);
                    f(x)
                }
            });
            self.right.select(x, |x| {
                let j = x.node();
                if !seen.contains(&(j as *const Json)) {
                    seen.insert(j as *const Json);
                    f(x)
                }
            })
        })
    }
}

pub struct Intersect<I,S,T> {
    inner: I,
    left: S,
    right: T
}

impl<I:Selector,S:Selector,T:Selector> Selector for Intersect<I,S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen_left = hash_set::HashSet::new();
        let mut seen_right = hash_set::HashSet::new();
        self.inner.select(input, |x| {
            self.left.select(x, |x| {
                let j = x.node();
                seen_left.insert(j as *const Json);
                if seen_right.contains(&(j as *const Json)) {
                    f(x)
                }
            });
            self.right.select(x, |x| {
                let j = x.node();
                seen_right.insert(j as *const Json);
                if seen_left.contains(&(j as *const Json)) {
                    f(x)
                }
            })
        })
    }
}

pub struct Diff<I,S,T> {
    inner: I,
    left: S,
    right: T
}

// FIXME: this has bad asymptotic behavior
// The results of the inner select can't be cached
// because the path breadcrumbs have a lifetime that
// can't escape the callback
impl<I:Selector,S:Selector,T:Selector> Selector for Diff<I,S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut seen = hash_set::HashSet::new();
        self.inner.select(input, |x| {
            self.right.select(x, |x| {
                seen.insert(x.node() as *const Json);
            })
        });
        self.inner.select(input, |x| {
            self.left.select(x, |x| {
                if !seen.contains(&(x.node() as *const Json)) {
                    f(x)
                }
            })
        })
    }
}

pub struct AndSel<I,S,T> {
    inner: I,
    left: S,
    right: T
}

static SINGLETON: Json = Json::Boolean(true);

impl<I:Selector,S:Selector,T:Selector> Selector for AndSel<I,S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut found_left = false;
        let mut found_right = false;
        self.inner.select(input, |x| {
            self.left.select(x, |_| found_left = true);
            self.right.select(x, |_| found_right = true)
        });
        if found_left && found_right {
            f(input.descendant(&SINGLETON))
        }
    }
}

pub struct OrSel<I,S,T> {
    inner: I,
    left: S,
    right: T
}

impl<I:Selector,S:Selector,T:Selector> Selector for OrSel<I,S,T> {
    fn select<'a,'b,F>(&self, input: JsonPath<'a,'b>, mut f: F)
                       where F: for<'c> FnMut(JsonPath<'a,'c>) {
        let mut found_left = false;
        let mut found_right = false;
        self.inner.select(input, |x| {
            self.left.select(x, |_| found_left = true);
            self.right.select(x, |_| found_right = true)
        });
        if found_left || found_right {
            f(input.descendant(&SINGLETON))
        }
    }
}

/// Extension trait for `Json`
pub trait JsonExt {
    /// Run query
    ///
    /// Runs the query represented by the selector `s`
    /// against the JSON document, accumulating and
    /// returning the results in a new vector.
    fn query<S:Selector>(&self, s: S) -> Vec<&Json>;
}

impl JsonExt for Json {
    fn query<S:Selector>(&self, s: S) -> Vec<&Json> {
        let mut outvec = Vec::new();
        {
            s.select(JsonPath::root(self), |x| {
                outvec.push(x.node())
            });
        }
        
        outvec
    }
}

/// Create trivial selector
///
/// Creates a trivial selector which always selects
/// the current node.  This is the starting point of
/// all selector chains which build up more complex
/// query expressions.
#[inline]
pub fn node() -> Node {
    Node { _dummy: () }
}

/// Shorthand for `node().boolean()`
#[inline]
pub fn boolean() -> BooleanSel<Node> {
    node().boolean()
}

/// Shorthand for `node().uint64()`
#[inline]
pub fn uint64() -> U64Sel<Node> {
    node().uint64()
}

/// Shorthand for `node().int64()`
#[inline]
pub fn int64() -> I64Sel<Node> {
    node().int64()
}

/// Shorthand for `node().float64()`
#[inline]
pub fn float64() -> F64Sel<Node> {
    node().float64()
}

/// Shorthand for `node().string()`
#[inline]
pub fn string() -> StringSel<Node> {
    node().string()
}

/// Shorthand for `node().object()`
#[inline]
pub fn object() -> ObjectSel<Node> {
    node().object()
}

/// Shorthand for `node().list()`
#[inline]
pub fn list() -> ListSel<Node> {
    node().list()
}

/// Shorthand for `node().null()`
#[inline]
pub fn null() -> NullSel<Node> {
    node().null()
}

/// Shorthand for `node().child()`
#[inline]
pub fn child() -> Child<Node> {
    node().child()
}

/// Shorthand for `node().parent()`
#[inline]
pub fn parent() -> Parent<Node> {
    node().parent()
}

/// Shorthand for `node().descend()`
#[inline]
pub fn descend() -> Descend<Node> {
    node().descend()
}

/// Shorthand for `node().ascend()`
#[inline]
pub fn ascend() -> Ascend<Node> {
    node().ascend()
}

/// Shorthand for `node().at(index)`
#[inline]
pub fn at(index: uint) -> At<Node> {
    node().at(index)
}

/// Shorthand for `node().key(name)`
#[inline]
pub fn key<'a>(name: &'a str) -> Key<'a, Node> {
    node().key(name)
}

/// Shorthand for `node().wherein(filter)`
#[inline]
pub fn wherein<T:Selector>(filter: T) -> Wherein<Node,T> {
    node().wherein(filter)
}

/// Shorthand for `node().intersect(left, right)`
#[inline]
pub fn intersect<T1:Selector,T2:Selector>(left: T1, right: T2) -> Intersect<Node,T1,T2> {
    node().intersect(left, right)
}

/// Shorthand for `node().union(left, right)`
#[inline]
pub fn union<T1:Selector,T2:Selector>(left: T1, right: T2) -> Union<Node,T1,T2> {
    node().union(left, right)
}

/// Shorthand for `node().diff(left, right)`
#[inline]
pub fn diff<T1:Selector,T2:Selector>(left: T1, right: T2) -> Diff<Node,T1,T2> {
    node().diff(left, right)
}

/// Shorthand for `node().and(left, right)`
#[inline]
pub fn and<T1:Selector,T2:Selector>(left: T1, right: T2) -> AndSel<Node,T1,T2> {
    node().and(left, right)
}

/// Shorthand for `node().or(left, right)`
#[inline]
pub fn or<T1:Selector,T2:Selector>(left: T1, right: T2) -> OrSel<Node,T1,T2> {
    node().or(left, right)
}

#[cfg(test)]
mod test {
    use super::{child,wherein,Selector,JsonExt};
    use serialize::json;

    #[test]
    fn parent_unique() {
        let json = json::from_str(r#"[{},{},{},{}]"#).unwrap();

        let matches = json.query(child().parent());
        assert_eq!(matches.len(), 1);

        let matches = json.query(child().parent().child());
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn ascend_unique() {
        let json = json::from_str(r#"[[{}],[{}],[{}],[{}]]"#).unwrap();

        let matches = json.query(child().child().ascend());
        assert_eq!(matches.len(), 5);
    }

    #[test]
    fn union_unique() {
        let json = json::from_str(r#"[[1],[2],[3],[1,2]]"#).unwrap();

        let matches = json.query(
            child().union(
                wherein(child().uint64().equals(1)),
                wherein(child().uint64().equals(2))));
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn match_null() {
        let json = json::from_str(r#"[{},null,{},null,{}]"#).unwrap();

        let matches = json.query(child().null());
        assert_eq!(matches.len(), 2);
    }
}
