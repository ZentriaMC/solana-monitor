use std::str::FromStr;

use http::Uri;

use crate::BoxError;

#[derive(Clone)]
pub struct IdUrlPair(pub (String, Uri));

impl FromStr for IdUrlPair {
    type Err = BoxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, '=');

        let id = split.next().ok_or("missing id")?.to_string();
        let uri: Uri = split.next().ok_or("missing uri")?.parse()?;

        Ok(Self((id, uri)))
    }
}

impl std::fmt::Debug for IdUrlPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.0 .0, self.0 .1)
    }
}
