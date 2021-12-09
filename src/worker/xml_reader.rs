use std::str::FromStr;
use xmltree::{Element, XMLNode};

pub fn get_el_child_text(el: &Element, name: &str) -> String {
    el.get_child(name).unwrap().get_text().unwrap().to_string()
}
pub fn get_el_child_text_as<T: FromStr>(el: &Element, name: &str) -> Result<T, T::Err> {
    get_el_child_text(el, name).parse()
}

pub fn get_node_child_text(node: &XMLNode, name: &str) -> String {
    get_el_child_text(node.as_element().unwrap(), name)
}
pub fn get_node_child_text_as<T: FromStr>(node: &XMLNode, name: &str) -> Result<T, T::Err> {
    get_node_child_text(node, name).parse()
}
