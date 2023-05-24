use alloc::string::String;
use alloc::vec::Vec;

pub fn canonicalize(path: &str) -> String {
    let mut buf = String::new();
    let is_absolute = path.starts_with('/');
    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                while !buf.is_empty() {
                    if buf == "/" {
                        break;
                    }
                    let c = buf.pop().unwrap();
                    if c == '/' {
                        break;
                    }
                }
            }
            _ => {
                if buf.is_empty() {
                    if is_absolute {
                        buf.push('/');
                    }
                } else if &buf[buf.len() - 1..] != "/" {
                    buf.push('/');
                }
                buf.push_str(part);
            }
        }
    }
    if is_absolute && buf.is_empty() {
        buf.push('/');
    }
    buf
}

pub fn split_path(path: &str) -> Vec<String> {
    let processed_path = canonicalize(path);
    let mut names: Vec<String> = Vec::new();

    for name in processed_path.split("/") {
        names.push(String::from(name))
    }
    names
}

pub fn split_parent_name(path: &str) -> (Option<String>, String) {
    assert!(!path.is_empty());
    let names = split_path(path);
    if names.len() == 1 {
        return (None, names[0].clone());
    } else {
        let parent = names[0..names.len()-1].join("/");
        let name = names[names.len()-1].clone();
        return (if parent.is_empty() { None } else { Some(parent) }, name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_canonicalize() {
        assert_eq!(canonicalize(""), "");
        assert_eq!(canonicalize("///"), "/");
        assert_eq!(canonicalize("//a//.//b///c//"), "/a/b/c");
        assert_eq!(canonicalize("/a/../"), "/");
        assert_eq!(canonicalize("/a/../..///"), "/");
        assert_eq!(canonicalize("a/../"), "");
        assert_eq!(canonicalize("a/..//.."), "");
        assert_eq!(canonicalize("././a"), "a");
        assert_eq!(canonicalize(".././a"), "a");
        assert_eq!(canonicalize("/././a"), "/a");
        assert_eq!(canonicalize("/abc/../abc"), "/abc");
        assert_eq!(canonicalize("/test"), "/test");
        assert_eq!(canonicalize("/test/"), "/test");
        assert_eq!(canonicalize("test/"), "test");
        assert_eq!(canonicalize("test"), "test");
        assert_eq!(canonicalize("/test//"), "/test");
        assert_eq!(canonicalize("/test/foo"), "/test/foo");
        assert_eq!(canonicalize("/test/foo/"), "/test/foo");
        assert_eq!(canonicalize("/test/foo/bar"), "/test/foo/bar");
        assert_eq!(canonicalize("/test/foo/bar//"), "/test/foo/bar");
        assert_eq!(canonicalize("/test//foo/bar//"), "/test/foo/bar");
        assert_eq!(canonicalize("/test//./foo/bar//"), "/test/foo/bar");
        assert_eq!(canonicalize("/test//./.foo/bar//"), "/test/.foo/bar");
        assert_eq!(canonicalize("/test//./..foo/bar//"), "/test/..foo/bar");
        assert_eq!(canonicalize("/test//./../foo/bar//"), "/foo/bar");
        assert_eq!(canonicalize("/test/../foo"), "/foo");
        assert_eq!(canonicalize("/test/bar/../foo"), "/test/foo");
        assert_eq!(canonicalize("../foo"), "foo");
        assert_eq!(canonicalize("../foo/"), "foo");
        assert_eq!(canonicalize("/../foo"), "/foo");
        assert_eq!(canonicalize("/../foo/"), "/foo");
        assert_eq!(canonicalize("/../../foo"), "/foo");
        assert_eq!(canonicalize("/bleh/../../foo"), "/foo");
        assert_eq!(canonicalize("/bleh/bar/../../foo"), "/foo");
        assert_eq!(canonicalize("/bleh/bar/../../foo/.."), "/");
        assert_eq!(canonicalize("/bleh/bar/../../foo/../meh"), "/meh");
    }
}
