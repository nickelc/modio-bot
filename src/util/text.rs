use std::fmt;

#[derive(Debug)]
pub struct ContentBuilder {
    limit: usize,
    pub content: Vec<String>,
}

impl ContentBuilder {
    pub fn new(limit: usize) -> Self {
        Self {
            content: vec![],
            limit,
        }
    }
}

impl Default for ContentBuilder {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl IntoIterator for ContentBuilder {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.content.into_iter()
    }
}

impl fmt::Write for ContentBuilder {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + s.len() > self.limit {
                    self.content.push(String::from(s));
                } else {
                    current.push_str(s);
                }
            }
            None => {
                self.content.push(String::from(s));
            }
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + c.len_utf8() > self.limit {
                    self.content.push(c.to_string());
                } else {
                    current.push(c);
                }
            }
            None => self.content.push(c.to_string()),
        }
        Ok(())
    }
}

pub mod mention {
    use std::fmt;

    use twilight_model::id::{marker::RoleMarker, Id};

    use crate::db::types::RoleId;

    pub struct Display<T>(T);

    pub trait Mention<T> {
        fn mention(&self) -> Display<T>;
    }

    impl Mention<Id<RoleMarker>> for RoleId {
        fn mention(&self) -> Display<Id<RoleMarker>> {
            Display(self.0)
        }
    }

    impl fmt::Display for Display<Id<RoleMarker>> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("<@&")?;
            fmt::Display::fmt(&self.0, f)?;
            f.write_str(">")
        }
    }
}

pub fn strip_html_tags<S: AsRef<str>>(input: S) -> String {
    use html5ever::tendril::TendrilSink;
    use html5ever::{parse_document, ParseOpts};
    use sink::TextOnly;

    parse_document(TextOnly::default(), ParseOpts::default()).one(input.as_ref())
}

mod sink {
    use std::borrow::Cow;
    use std::cell::RefCell;
    use std::rc::Rc;

    use html5ever::tendril::StrTendril;
    use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
    use html5ever::{Attribute, ExpandedName, QualName};

    #[derive(Default)]
    pub struct TextOnly {
        text: RefCell<String>,
    }

    #[derive(Clone)]
    pub struct Node {
        data: NodeData,
    }

    impl Node {
        fn new(data: NodeData) -> Rc<Self> {
            Rc::new(Self { data })
        }
    }

    #[derive(Clone)]
    enum NodeData {
        Document,
        Comment,
        ProcessingInformation,
        Element { name: QualName },
    }

    type Handle = Rc<Node>;

    impl TreeSink for TextOnly {
        type Handle = Handle;
        type ElemName<'a> = ExpandedName<'a>;
        type Output = String;

        fn finish(self) -> Self::Output {
            self.text.into_inner()
        }

        fn parse_error(&self, _msg: Cow<'static, str>) {}

        fn get_document(&self) -> Self::Handle {
            Node::new(NodeData::Document)
        }

        fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
            match &target.data {
                NodeData::Element { name } => name.expanded(),
                _ => panic!("not an element!"),
            }
        }

        fn create_element(
            &self,
            name: QualName,
            _attrs: Vec<Attribute>,
            _flags: ElementFlags,
        ) -> Self::Handle {
            Node::new(NodeData::Element { name })
        }

        fn create_comment(&self, _text: StrTendril) -> Self::Handle {
            Node::new(NodeData::Comment)
        }

        fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
            Node::new(NodeData::ProcessingInformation)
        }

        fn append_doctype_to_document(
            &self,
            _name: StrTendril,
            _public_id: StrTendril,
            _system_id: StrTendril,
        ) {
        }

        fn append(&self, _parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
            if let NodeOrText::AppendText(text) = &child {
                self.text.borrow_mut().push_str(text);
            }
        }

        fn append_based_on_parent_node(
            &self,
            _element: &Self::Handle,
            _prev_element: &Self::Handle,
            child: NodeOrText<Self::Handle>,
        ) {
            if let NodeOrText::AppendText(text) = &child {
                self.text.borrow_mut().push_str(text);
            }
        }

        fn append_before_sibling(
            &self,
            _sibling: &Self::Handle,
            _new_node: NodeOrText<Self::Handle>,
        ) {
            // This would be called for `InsertionPoint::BeforeSibling` but this enum variant is
            // currently not constructed in `html5ever`'s code.
            unimplemented!()
        }

        fn get_template_contents(&self, _target: &Self::Handle) -> Self::Handle {
            Node::new(NodeData::Document)
        }

        fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
            Rc::ptr_eq(x, y)
        }

        fn set_quirks_mode(&self, _mode: QuirksMode) {}

        fn add_attrs_if_missing(&self, _target: &Self::Handle, _attrs: Vec<Attribute>) {}

        fn remove_from_parent(&self, _target: &Self::Handle) {}

        fn reparent_children(&self, _node: &Self::Handle, _new_parent: &Self::Handle) {}

        fn clone_subtree(&self, node: &Self::Handle) -> Self::Handle {
            Node::new(node.data.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::{strip_html_tags, ContentBuilder};

    #[test]
    fn content_builder() {
        let mut c = ContentBuilder::new(20);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 1);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(5));

        let _ = c.write_char('f');
        let _ = c.write_char('o');
        let _ = c.write_char('o');
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[1], "foo".repeat(6));

        let _ = c.write_str("foobar");
        assert_eq!(c.content.len(), 3);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(6));
        assert_eq!(c.content[2], "foobar");
    }

    #[test]
    fn test_strip_html_tags() {
        let input = "aaa<br/>";
        assert_eq!("aaa", strip_html_tags(input));

        let input = "aaa<br/> bbb";
        assert_eq!("aaa bbb", strip_html_tags(input));

        let input = "- aaa\n- bbb\n - ccc\n";
        assert_eq!(input, strip_html_tags(input));

        let input = "<div>- aaa\n- bbb\n - ccc\n</div>";
        assert_eq!("- aaa\n- bbb\n - ccc\n", strip_html_tags(input));
    }
}
