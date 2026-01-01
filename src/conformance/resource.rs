use std::sync::LazyLock;

use anyhow::anyhow;
use anyhow::Result;
use bon::builder;
use bon::Builder;
use regex::Captures;
use regex::Regex;

/// The regex for resource files the specification.
static RESOURCE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    const PATTERN: &str = concat!(
        "(?is)", // Turn on `i` and `s` options.
        r"<details>\s*",
        r"<summary>\s*",
        r"Resource: (.+?)\s*```\w*\s*(.+?)```\s*",
        r"</summary>\s*",
        r"</details>"
    );

    Regex::new(PATTERN).unwrap()
});

/// A resource file.
#[derive(Builder, Debug)]
#[builder(builder_type = Builder)]
pub struct Resource {
    /// The file name.
    filename: String,

    /// The source of the resource file.
    src: String,
}

impl Resource {
    /// Gets the file name.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Gets the source of the resource file.
    pub fn src(&self) -> &str {
        &self.src
    }
}

/// A set of resource files.
#[derive(Debug)]
pub struct Resources(Vec<Resource>);

impl Resources {
    /// Turns a markdown specification into a set of resources.
    pub fn compile<S: AsRef<str>>(contents: S) -> Result<Self> {
        let contents = contents.as_ref();

        RESOURCE_REGEX
            .captures_iter(contents)
            .map(build_resource)
            .collect::<Result<Self, _>>()
    }
}

impl Resources {
    /// Generates an iterator for the resources.
    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.0.iter()
    }
}

impl FromIterator<Resource> for Resources {
    fn from_iter<T: IntoIterator<Item = Resource>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Builds a resource from a set of captures.
fn build_resource(captures: Captures<'_>) -> Result<Resource> {
    let filename = required_string(&captures, 1, "filename")?;
    let src = required_string(&captures, 2, "source")?;
    Ok(Resource::builder().filename(filename).src(src).build())
}

/// Parses a _required_ group within a test.
fn required_string(captures: &Captures, index: usize, name: &str) -> Result<String> {
    captures
        .get(index)
        .ok_or_else(|| {
            anyhow!(
                "unable to parse {} from resource:\n\n{}",
                name,
                captures.get(0).unwrap().as_str()
            )
        })
        .map(|v| v.as_str().to_owned())
}
