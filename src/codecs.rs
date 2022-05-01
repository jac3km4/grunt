pub mod rfc3339_date {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description, OffsetDateTime};

    pub fn serialize<S>(date: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = date
            .format(&format_description::well_known::Rfc3339)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = Deserialize::<'de>::deserialize(deserializer)?;
        OffsetDateTime::parse(str, &format_description::well_known::Rfc3339)
            .map_err(serde::de::Error::custom)
    }
}
