#![crate_type = "rlib"]
#![crate_id = "jlens#0.0.1"]
#![feature(globs)]

extern crate serialize;

use serialize::json;
use std::collections::hashmap;

pub struct Path<'a,'b>(&'a json::Json, Option<&'b Path<'a,'b>>);

pub trait Selector {
    fn select<'a,'b>(&self, input: Path<'a,'b>, f: <'c>|Path<'a,'c>|);
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
                Path(&json::Object(box ref m),_) => {
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
                Path(&json::Object(box ref m),_) => {
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
        self.inner.select(input, |x| {
            match x {
                Path(_,Some(&p)) => f(p),
                _ => ()
            }
        })
    }
}

pub struct Descend<S> {
    inner: S
}

fn descend_helper<'a,'b>(input@Path(j,_): Path<'a,'b>,
                         seen: &mut hashmap::HashSet<*json::Json>,
                         f: <'c>|Path<'a,'c>|) {
    if !seen.contains(&(j as *json::Json)) {
        seen.insert(j as *json::Json);
        match j {
            &json::Object(box ref m) => {
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
        self.inner.select(input, |mut n| {
            loop {
                match n {
                    Path(_,Some(&x)) => {
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
                if !seen.contains(&(j as *json::Json)) {
                    seen.insert(j as *json::Json);
                    f(x)
                }
            });
            self.right.select(x, |x@Path(j,_)| {
                if !seen.contains(&(j as *json::Json)) {
                    seen.insert(j as *json::Json);
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
                seen_left.insert(j as *json::Json);
                if seen_right.contains(&(j as *json::Json)) {
                    f(x)
                }
            });
            self.right.select(x, |x@Path(j,_)| {
                seen_right.insert(j as *json::Json);
                if seen_left.contains(&(j as *json::Json)) {
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
                seen.insert(j as *json::Json);
            })
        });
        self.inner.select(input, |x| {
            self.left.select(x, |x@Path(j,_)| {
                if !seen.contains(&(j as *json::Json)) {
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

pub trait SelectorExt {
    fn boolean(self) -> Boolean<Self>;
    fn number(self) -> Number<Self>;
    fn string(self) -> String<Self>;
    fn object(self) -> Object<Self>;
    fn list(self) -> List<Self>;
    fn at(self, index: uint) -> At<Self>;
    fn key<'a>(self, name: &'a str) -> Key<'a, Self>;
    fn child(self) -> Child<Self>;
    fn parent(self) -> Parent<Self>;
    fn descend(self) -> Descend<Self>;
    fn ascend(self) -> Ascend<Self>;
    fn where<T:Selector>(self, filter: T) -> Where<Self,T>;
    fn intersect<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Intersect<Self,T1,T2>;
    fn union<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Union<Self,T1,T2>;
    fn diff<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Diff<Self,T1,T2>;
    fn and<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> And<Self,T1,T2>;
    fn or<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Or<Self,T1,T2>;
}

impl<S:Selector> SelectorExt for S {
    fn boolean(self) -> Boolean<S> {
        Boolean { inner: self }
    }
    fn number(self) -> Number<S> {
        Number { inner: self }
    }
    fn string(self) -> String<S> {
        String { inner: self }
    }
    fn object(self) -> Object<S> {
        Object { inner: self }
    }
    fn list(self) -> List<S> {
        List { inner: self }
    }
    fn at(self, index: uint) -> At<S> {
        At { inner: self, index: index }
    }
    fn key<'a>(self, name: &'a str) -> Key<'a, S> {
        Key { inner: self, name: name }
    }
    fn child(self) -> Child<S> {
        Child { inner: self }
    }
    fn parent(self) -> Parent<S> {
        Parent { inner: self }
    }
    fn descend(self) -> Descend<S> {
        Descend { inner: self }
    }
    fn ascend(self) -> Ascend<S> {
        Ascend { inner: self }
    }
    fn where<T:Selector>(self, filter: T) -> Where<S,T> {
        Where { inner: self, filter: filter }
    }
    fn union<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Union<S,T1,T2> {
        Union { inner: self, left: left, right: right }
    }
    fn intersect<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Intersect<S,T1,T2> {
        Intersect { inner: self, left: left, right: right }
    }
    fn diff<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Diff<S,T1,T2> {
        Diff { inner: self, left: left, right: right }
    }
    fn and<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> And<S,T1,T2> {
        And { inner: self, left: left, right: right }
    }
    fn or<T1:Selector,T2:Selector>(self, left: T1, right: T2) -> Or<S,T1,T2> {
        Or { inner: self, left: left, right: right }
    }
}

pub trait JsonExt {
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

pub fn node() -> Node {
    Node { _dummy: () }
}

pub fn boolean() -> Boolean<Node> {
    node().boolean()
}

pub fn number() -> Number<Node> {
    node().number()
}

pub fn string() -> String<Node> {
    node().string()
}

pub fn object() -> Object<Node> {
    node().object()
}

pub fn list() -> List<Node> {
    node().list()
}

pub fn child() -> Child<Node> {
    node().child()
}

pub fn parent() -> Parent<Node> {
    node().parent()
}

pub fn descend() -> Descend<Node> {
    node().descend()
}

pub fn ascend() -> Ascend<Node> {
    node().ascend()
}

pub fn at(index: uint) -> At<Node> {
    node().at(index)
}

pub fn key<'a>(name: &'a str) -> Key<'a, Node> {
    node().key(name)
}

pub fn where<T:Selector>(filter: T) -> Where<Node,T> {
    node().where(filter)
}

pub fn intersect<T1:Selector,T2:Selector>(left: T1, right: T2) -> Intersect<Node,T1,T2> {
    node().intersect(left, right)
}

pub fn union<T1:Selector,T2:Selector>(left: T1, right: T2) -> Union<Node,T1,T2> {
    node().union(left, right)
}

pub fn diff<T1:Selector,T2:Selector>(left: T1, right: T2) -> Diff<Node,T1,T2> {
    node().diff(left, right)
}

pub fn and<T1:Selector,T2:Selector>(left: T1, right: T2) -> And<Node,T1,T2> {
    node().and(left, right)
}

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
}
