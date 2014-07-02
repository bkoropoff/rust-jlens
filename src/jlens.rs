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
//! // or the number 42
//! let matches = json.query(
//!     list().child().where(
//!         key("foo").list().child().or(
//!             string().equals("Hello, world!"),
//!             number().equals(42f64))));
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
//! ```
//!
//! The `JsonExt` trait provides a convenience method on `Json`
//! objects which runs a selector and returns a `Vec<&'self Json>` of
//! results.

#![crate_type = "rlib"]
#![crate_id = "jlens#0.0.1"]
#![feature(globs)]

extern crate serialize;

use serialize::json;
use std::collections::hashmap;

/// JSON node path
///
/// Represents a path to a JSON node.  It consists of
/// a reference to the node in question and an optional
/// reference to a path to the parent node.  This
/// optional parent path should be `None` only for the root
/// node of a `Json` object.
pub struct Path<'a,'b>(pub &'a json::Json, pub Option<&'b Path<'a,'b>>);

/// JSON selector trait
///
/// Implementors of this trait select nodes from `Json`
/// objects according to some criteria.
pub trait Selector {
    /// Select matching nodes
    ///
    /// Given the path to a single node, `input`, this
    /// method should identify nodes to be selected and
    /// invoke the closure `f` with a path to each.
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|);

    /// Select current node if it is a `json::Boolean`
    fn boolean(self) -> Boolean<Self> {
        Boolean { inner: self }
    }

    /// Select current node if it is a `json::Number`
    fn number(self) -> Number<Self> {
        Number { inner: self }
    }

    /// Select current node if it is a `json::String`
    fn string(self) -> String<Self> {
        String { inner: self }
    }

    /// Select current node if it is a `json::Object`
    fn object(self) -> Object<Self> {
        Object { inner: self }
    }

    /// Select current node if it is a `json::List`
    fn list(self) -> List<Self> {
        List { inner: self }
    }

    /// Select current node if it is a `json::Null`
    fn null(self) -> Null<Self> {
        Null { inner: self }
    }

    /// Select list element
    ///
    /// If the current node is a `json::List` of at
    /// least `index + 1` elements, selects the element
    /// at `index`.  Otherwise no nodes are selected.
    fn at(self, index: uint) -> At<Self> {
        At { inner: self, index: index }
    }

    /// Select object value for key
    ///
    /// If the current node is a `json::Object` that contains
    /// the key `name`, its value is selected.  Otherwise no
    /// nodes are selected.
    fn key<'a>(self, name: &'a str) -> Key<'a, Self> {
        Key { inner: self, name: name }
    }

    /// Select children of current node
    ///
    /// Selects all immediate child nodes of the current node:
    /// all elements of a `json::List`, or all values of a
    /// `json::Object`.
    fn child(self) -> Child<Self> {
        Child { inner: self }
    }

    /// Select parent of current node if it is not the root
    fn parent(self) -> Parent<Self> {
        Parent { inner: self }
    }

    /// Select descendents of current node
    ///
    /// Selects all child nodes of the current node and all their
    /// children, recursively.
    fn descend(self) -> Descend<Self> {
        Descend { inner: self }
    }

    /// Select ancestors of current node
    ///
    /// Selects the parent, grandparent, etc. of the current node
    /// up to the root of the tree.
    fn ascend(self) -> Ascend<Self> {
        Ascend { inner: self }
    }

    /// Select current node based on filter
    ///
    /// Runs the selector `filter` on the current node.  If it selects
    /// any nodes, the current node is selected.  If it does not select
    /// any nodes, no nodes are selected.
    fn where<T:Selector>(self, filter: T) -> Where<Self,T> {
        Where { inner: self, filter: filter }
    }

    /// Select union of two selectors
    ///
    /// Runs `left` and `right` on the current node and selects
    /// nodes which are selected by either.
    fn union<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Union<Self,T1,T2> {
        Union { inner: self, left: left, right: right }
    }

    /// Select intersection of two selectors
    ///
    /// Runs `left` and `right` on the current node and selects
    /// nodes which are selected by both.
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
    fn diff<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Diff<Self,T1,T2> {
        Diff { inner: self, left: left, right: right }
    }

    /// Select logical-and of two selectors
    ///
    /// Runs `left` and `right` on the current node and
    /// selects an arbitrary node if both selected at
    /// least one node themselves.  This is useful for
    /// encoding logical-and conditions for `which`.
    fn and<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> And<Self,T1,T2> {
        And { inner: self, left: left, right: right }
    }

    /// Select logical-or of two selectors
    ///
    /// Runs `left` and `right` on the current node and
    /// selects an arbitrary node if either selected at
    /// least one node themselves.  This is useful for
    /// encoding logical-and conditions for `which`.
    fn or<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Or<Self,T1,T2> {
        Or { inner: self, left: left, right: right }
    }
}

pub struct Node {
    _dummy: ()
}

impl<'f> Selector for Node {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        f(input)
    }
}

pub struct Object<S> {
    inner: S
}

impl<S:Selector> Selector for Object<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Object(..),_) => f(x),
                _ => ()
            }
        })
    }
}

pub struct List<S> {
    inner: S
}

impl<S:Selector> Selector for List<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::List(..),_) => f(x),
                _ => ()
            }
        })
    }
}

pub struct String<S> {
    inner: S
}

pub struct StringEquals<'a,S> {
    inner: S,
    comp: &'a str
}

impl<S:Selector> String<S> {
    /// Select current `json::String` node if it is equal to `comp`
    pub fn equals<'a,'b>(self, comp: &'a str) -> StringEquals<'a,S> {
        let String { inner: inner } = self;
        StringEquals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for String<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::String(..),_) => f(x),
                _ => ()
            }
        })
    }
}

impl<'a,S:Selector> Selector for StringEquals<'a,S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::String(ref s),_) if self.comp.equiv(s) => f(x),
                _ => ()
            }
        })
    }
}

pub struct Boolean<S> {
    inner: S
}

pub struct BooleanEquals<S> {
    inner: S,
    comp: bool
}

impl<S:Selector> Boolean<S> {
    /// Select current `json::Boolean` node if it is equal to `comp`
    pub fn equals(self, comp: bool) -> BooleanEquals<S> {
        let Boolean { inner: inner } = self;
        BooleanEquals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for Boolean<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Boolean(..),_) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for BooleanEquals<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Boolean(b),_) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct Number<S> {
    inner: S
}

pub struct NumberEquals<S> {
    inner: S,
    comp: f64
}

impl<S:Selector> Number<S> {
    pub fn equals(self, comp: f64) -> NumberEquals<S> {
        let Number { inner: inner } = self;
        NumberEquals { inner: inner, comp: comp }
    }
}

impl<S:Selector> Selector for Number<S> {
    /// Select current `json::Number` node if it is equal to `comp`
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Number(..),_) => f(x),
                _ => ()
            }
        })
    }
}

impl<S:Selector> Selector for NumberEquals<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Number(b),_) if b == self.comp => f(x),
                _ => ()
            }
        })
    }
}

pub struct Null<S> {
    inner: S
}

impl<S:Selector> Selector for Null<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Null,_) => f(x),
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::List(ref v),_) => {
                    if v.len() > self.index {
                        f(Path(v.get(self.index),Some(&x)))
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Object(ref m),_) => {
                    match m.find(&self.name.to_string()) {
                        Some(e) => f(Path(e,Some(&x))),
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        self.inner.select(input, |x| {
            match x {
                Path(&json::Object(ref m),_) => {
                    for (_,child) in m.iter() {
                        f(Path(child,Some(&x)))
                    }
                },
                Path(&json::List(ref v),_) => {
                    for child in v.iter() {
                        f(Path(child,Some(&x)))
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen = hashmap::HashSet::new();
        self.inner.select(input, |x| {
            match x {
                Path(_,Some(&p@Path(j,_))) if !seen.contains(&(j as *const json::Json)) => {
                    seen.insert(j as *const json::Json);
                    f(p)
                }
                _ => ()
            }
        })
    }
}

pub struct Descend<S> {
    inner: S
}

fn descend_helper<'a,'b>(input@Path(j,_): Path<'a,'b>,
                         seen: &mut hashmap::HashSet<*const json::Json>,
                         f: <'c>|Path<'a,'c>|) {
    if !seen.contains(&(j as *const json::Json)) {
        seen.insert(j as *const json::Json);
        match j {
            &json::Object(ref m) => {
                for (_,c) in m.iter() {
                    let inner = Path(c,Some(&input));
                    f(inner);
                    descend_helper(inner, seen, |x| f(x))
                }
            },
            &json::List(ref v) => {
                for c in v.iter() {
                    let inner = Path(c,Some(&input));
                    f(inner);
                    descend_helper(inner, seen, |x| f(x))
                }
            },
            _ => ()
        }
    }
}

impl<S:Selector> Selector for Descend<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen = hashmap::HashSet::new();
        self.inner.select(input, |x| {
            descend_helper(x, &mut seen, |x| f(x))
        })
    }
}

pub struct Ascend<S> {
    inner: S
}

impl<S:Selector> Selector for Ascend<S> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen = hashmap::HashSet::new();
        self.inner.select(input, |mut n| {
            loop {
                match n {
                    Path(_,Some(&x@Path(j,_))) if !seen.contains(&(j as *const json::Json)) => {
                        seen.insert(j as *const json::Json);
                        f(x);
                        n = x;
                    },
                    _ => break
                }
            }
        })
    }
}

pub struct Where<S,T> {
    inner: S,
    filter: T
}

impl<S:Selector,T:Selector> Selector for Where<S,T> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen = hashmap::HashSet::new();
        self.inner.select(input, |x| {
            self.left.select(x, |x@Path(j,_)| {
                if !seen.contains(&(j as *const json::Json)) {
                    seen.insert(j as *const json::Json);
                    f(x)
                }
            });
            self.right.select(x, |x@Path(j,_)| {
                if !seen.contains(&(j as *const json::Json)) {
                    seen.insert(j as *const json::Json);
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen_left = hashmap::HashSet::new();
        let mut seen_right = hashmap::HashSet::new();
        self.inner.select(input, |x| {
            self.left.select(x, |Path(j,_)| {
                seen_left.insert(j as *const json::Json);
                if seen_right.contains(&(j as *const json::Json)) {
                    f(x)
                }
            });
            self.right.select(x, |x@Path(j,_)| {
                seen_right.insert(j as *const json::Json);
                if seen_left.contains(&(j as *const json::Json)) {
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
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut seen = hashmap::HashSet::new();
        self.inner.select(input, |x| {
            self.right.select(x, |Path(j,_)| {
                seen.insert(j as *const json::Json);
            })
        });
        self.inner.select(input, |x| {
            self.left.select(x, |x@Path(j,_)| {
                if !seen.contains(&(j as *const json::Json)) {
                    f(x)
                }
            })
        })
    }
}

pub struct And<I,S,T> {
    inner: I,
    left: S,
    right: T
}

static SINGLETON: json::Json = json::Boolean(true);

impl<I:Selector,S:Selector,T:Selector> Selector for And<I,S,T> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut found_left = false;
        let mut found_right = false;
        self.inner.select(input, |x| {
            self.left.select(x, |_| found_left = true);
            self.right.select(x, |_| found_right = true)
        });
        if found_left && found_right {
            f(Path(&SINGLETON, Some(&input)))
        }
    }
}

pub struct Or<I,S,T> {
    inner: I,
    left: S,
    right: T
}

impl<I:Selector,S:Selector,T:Selector> Selector for Or<I,S,T> {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|) {
        let mut found_left = false;
        let mut found_right = false;
        self.inner.select(input, |x| {
            self.left.select(x, |_| found_left = true);
            self.right.select(x, |_| found_right = true)
        });
        if found_left || found_right {
            f(Path(&SINGLETON, Some(&input)))
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
    fn query<'a,S:Selector>(&'a self, s: S) -> Vec<&'a json::Json>;
}

impl JsonExt for json::Json {
    fn query<'a,S:Selector>(&'a self, s: S) -> Vec<&'a json::Json> {
        let mut outvec = Vec::new();
        {
            let output = &mut outvec;
            s.select(Path(self,None), |Path(j,_)| {
                output.push(j)
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
pub fn node() -> Node {
    Node { _dummy: () }
}

/// Shorthand for `node().boolean()`
pub fn boolean() -> Boolean<Node> {
    node().boolean()
}

/// Shorthand for `node().number()`
pub fn number() -> Number<Node> {
    node().number()
}

/// Shorthand for `node().string()`
pub fn string() -> String<Node> {
    node().string()
}

/// Shorthand for `node().object()`
pub fn object() -> Object<Node> {
    node().object()
}

/// Shorthand for `node().list()`
pub fn list() -> List<Node> {
    node().list()
}

/// Shorthand for `node().null()`
pub fn null() -> Null<Node> {
    node().null()
}

/// Shorthand for `node().child()`
pub fn child() -> Child<Node> {
    node().child()
}

/// Shorthand for `node().parent()`
pub fn parent() -> Parent<Node> {
    node().parent()
}

/// Shorthand for `node().descend()`
pub fn descend() -> Descend<Node> {
    node().descend()
}

/// Shorthand for `node().ascend()`
pub fn ascend() -> Ascend<Node> {
    node().ascend()
}

/// Shorthand for `node().at(index)`
pub fn at(index: uint) -> At<Node> {
    node().at(index)
}

/// Shorthand for `node().key(name)`
pub fn key<'a>(name: &'a str) -> Key<'a, Node> {
    node().key(name)
}

/// Shorthand for `node().where(filter)`
pub fn where<T:Selector>(filter: T) -> Where<Node,T> {
    node().where(filter)
}

/// Shorthand for `node().intersect(left, right)`
pub fn intersect<T1:Selector,T2:Selector>(left: T1, right: T2) -> Intersect<Node,T1,T2> {
    node().intersect(left, right)
}

/// Shorthand for `node().union(left, right)`
pub fn union<T1:Selector,T2:Selector>(left: T1, right: T2) -> Union<Node,T1,T2> {
    node().union(left, right)
}

/// Shorthand for `node().diff(left, right)`
pub fn diff<T1:Selector,T2:Selector>(left: T1, right: T2) -> Diff<Node,T1,T2> {
    node().diff(left, right)
}

/// Shorthand for `node().and(left, right)`
pub fn and<T1:Selector,T2:Selector>(left: T1, right: T2) -> And<Node,T1,T2> {
    node().and(left, right)
}

/// Shorthand for `node().or(left, right)`
pub fn or<T1:Selector,T2:Selector>(left: T1, right: T2) -> Or<Node,T1,T2> {
    node().or(left, right)
}

#[cfg(test)]
mod test {
    use super::*;
    use serialize::json;

    #[test]
    fn basic() {
        // Test JSON document
        let json = json::from_str(
r#"
[
    {
        "foo": ["Hello, world!", 3.14, false]
    },
    {
        "foo": [42, true]
    },
    {
        "foo": "Nope"
    },
    {
        "bar": [42, "Hello, world!"]
    }
]
"#).unwrap();

        // Given a list, match all objects in it that
        // have a "foo" key where the value is a list
        // that contains either the string "Hello, world!"
        // or the number 42
        let matches = json.query(
            list().child().where(
                key("foo").list().child().or(
                    string().equals("Hello, world!"),
                    number().equals(42f64))));

        // Expected matches
        let match1 = json::from_str(
            r#"{"foo": ["Hello, world!", 3.14, false]}"#).unwrap();
        let match2 = json::from_str(
            r#"{"foo": [42, true]}"#).unwrap();

        assert_eq!(matches.len(), 2);
        assert!(matches.contains(& &match1));
        assert!(matches.contains(& &match2));
    }

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
                where(child().number().equals(1f64)),
                where(child().number().equals(2f64))));
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn null() {
        let json = json::from_str(r#"[{},null,{},null,{}]"#).unwrap();

        let matches = json.query(child().null());
        assert_eq!(matches.len(), 2);
    }
}
