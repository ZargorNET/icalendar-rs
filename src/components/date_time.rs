use std::str::FromStr;

use chrono::*;

use crate::{Property, ValueType};

const NAIVE_DATE_TIME_FORMAT: &str = "%Y%m%dT%H%M%S";
const UTC_DATE_TIME_FORMAT: &str = "%Y%m%dT%H%M%SZ";
const NAIVE_DATE_FORMAT: &str = "%Y%m%d";

// #[deprecated(note = "use `CalendarDateTime::from_str` if you can")]
pub(crate) fn parse_utc_date_time(s: &str) -> Option<DateTime<Utc>> {
    Utc.datetime_from_str(s, UTC_DATE_TIME_FORMAT).ok()
}

pub(crate) fn parse_naive_date_time(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, NAIVE_DATE_TIME_FORMAT).ok()
}

pub(crate) fn format_utc_date_time(utc_dt: DateTime<Utc>) -> String {
    utc_dt.format(UTC_DATE_TIME_FORMAT).to_string()
}

pub(crate) fn parse_duration(s: &str) -> Option<Duration> {
    iso8601::duration(s)
        .ok()
        .and_then(|iso| Duration::from_std(iso.into()).ok())
}

pub(crate) fn naive_date_to_property(date: NaiveDate, key: &str) -> Property {
    Property::new(key, &date.format(NAIVE_DATE_FORMAT).to_string())
        .append_parameter(ValueType::Date)
        .done()
}

/// Representation of various forms of `DATE-TIME` per
/// [RFC 5545, Section 3.3.5](https://tools.ietf.org/html/rfc5545#section-3.3.5)
///
/// Conversions from [chrono] types are provided in form of [From] implementations, see
/// documentation of individual variants.
///
/// In addition to readily implemented `FORM #1` and `FORM #2`, the RFC also specifies
/// `FORM #3: DATE WITH LOCAL TIME AND TIME ZONE REFERENCE`. This variant is not yet implemented.
/// Adding it will require adding support for `VTIMEZONE` and referencing it using `TZID`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalendarDateTime {
    /// `FORM #1: DATE WITH LOCAL TIME`: floating, follows current time-zone of the attendee.
    ///
    /// Conversion from [`chrono::NaiveDateTime`] results in this variant.
    Floating(NaiveDateTime),

    /// `FORM #2: DATE WITH UTC TIME`: rendered with Z suffix character.
    ///
    /// Conversion from [`chrono::DateTime<Utc>`](DateTime) results in this variant. Use
    /// `date_time.with_timezone(&Utc)` to convert `date_time` from arbitrary time zone to UTC.
    Utc(DateTime<Utc>),

    /// `FORM #3: DATE WITH LOCAL TIME AND TIME ZONE REFERENCE`: refers to a time zone definition.
    WithTimezone {
        /// The date and time in the given time zone.
        date_time: NaiveDateTime,
        /// The ID of the time zone definition in a VTIMEZONE calendar component.
        tzid: String,
    },
}

impl CalendarDateTime {
    /// this is not actually now, just a fixed date for testing
    #[cfg(test)]
    pub(crate) fn now() -> Self {
        NaiveDate::from_ymd_opt(2015, 10, 26)
            .unwrap()
            .and_hms_opt(1, 22, 00)
            .unwrap()
            .into()
    }

    pub(crate) fn from_property(property: &Property) -> Option<Self> {
        let value = property.value();
        if let Some(tzid) = property.params().get("TZID") {
            Some(Self::WithTimezone {
                date_time: NaiveDateTime::parse_from_str(value, NAIVE_DATE_TIME_FORMAT).ok()?,
                tzid: tzid.value().to_owned(),
            })
        } else if let Ok(naive_date_time) =
            NaiveDateTime::parse_from_str(value, NAIVE_DATE_TIME_FORMAT)
        {
            Some(naive_date_time.into())
        } else {
            Self::from_str(value).ok()
        }
    }

    pub(crate) fn to_property(&self, key: &str) -> Property {
        match self {
            CalendarDateTime::Floating(naive_dt) => {
                Property::new(key, &naive_dt.format(NAIVE_DATE_TIME_FORMAT).to_string())
            }
            CalendarDateTime::Utc(utc_dt) => Property::new(key, &format_utc_date_time(*utc_dt)),
            CalendarDateTime::WithTimezone { date_time, tzid } => {
                Property::new(key, &date_time.format(NAIVE_DATE_TIME_FORMAT).to_string())
                    .add_parameter("TZID", tzid)
                    .done()
            }
        }
    }

    pub(crate) fn from_utc_string(s: &str) -> Option<Self> {
        parse_utc_date_time(s).map(CalendarDateTime::Utc)
    }

    pub(crate) fn from_naive_string(s: &str) -> Option<Self> {
        parse_naive_date_time(s).map(CalendarDateTime::Floating)
    }
}

/// Converts from time zone-aware UTC date-time to [`CalendarDateTime::Utc`].
impl From<DateTime<Utc>> for CalendarDateTime {
    fn from(dt: DateTime<Utc>) -> Self {
        Self::Utc(dt)
    }
}

/// Converts from time zone-less date-time to [`CalendarDateTime::Floating`].
impl From<NaiveDateTime> for CalendarDateTime {
    fn from(dt: NaiveDateTime) -> Self {
        Self::Floating(dt)
    }
}

impl FromStr for CalendarDateTime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CalendarDateTime::from_utc_string(s)
            .or_else(|| CalendarDateTime::from_naive_string(s))
            .ok_or(())
    }
}

/// Either a `DATE-TIME` or a `DATE`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatePerhapsTime {
    /// A `DATE-TIME` property.
    DateTime(CalendarDateTime),
    /// A `DATE` property.
    Date(NaiveDate),
}

impl DatePerhapsTime {
    pub(crate) fn from_property(property: &Property) -> Option<Self> {
        if property.value_type() == Some(ValueType::Date) {
            Some(
                NaiveDate::parse_from_str(property.value(), NAIVE_DATE_FORMAT)
                    .ok()?
                    .into(),
            )
        } else {
            Some(CalendarDateTime::from_property(property)?.into())
        }
    }

    pub(crate) fn to_property(&self, key: &str) -> Property {
        match self {
            Self::DateTime(date_time) => date_time.to_property(key),
            Self::Date(date) => naive_date_to_property(*date, key),
        }
    }
}

impl From<CalendarDateTime> for DatePerhapsTime {
    fn from(dt: CalendarDateTime) -> Self {
        Self::DateTime(dt)
    }
}

impl From<DateTime<Utc>> for DatePerhapsTime {
    fn from(dt: DateTime<Utc>) -> Self {
        Self::DateTime(dt.into())
    }
}

#[allow(deprecated)]
impl From<Date<Utc>> for DatePerhapsTime {
    fn from(dt: Date<Utc>) -> Self {
        Self::Date(dt.naive_utc())
    }
}

impl From<NaiveDateTime> for DatePerhapsTime {
    fn from(dt: NaiveDateTime) -> Self {
        Self::DateTime(dt.into())
    }
}

impl From<NaiveDate> for DatePerhapsTime {
    fn from(date: NaiveDate) -> Self {
        Self::Date(date)
    }
}

#[cfg(feature = "parser")]
impl TryFrom<&crate::parser::Property<'_>> for DatePerhapsTime {
    type Error = &'static str;

    fn try_from(value: &crate::parser::Property) -> Result<Self, Self::Error> {
        let val = value.val.as_ref();

        // UTC is here first because lots of fields MUST be UTC, so it should,
        // in practice, be more common that others.
        if let Ok(utc_dt) = Utc.datetime_from_str(val, "%Y%m%dT%H%M%SZ") {
            return Ok(Self::DateTime(CalendarDateTime::Utc(utc_dt)));
        };

        if let Ok(naive_date) = NaiveDate::parse_from_str(val, "%Y%m%d") {
            return Ok(Self::Date(naive_date));
        };

        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%S") {
            if let Some(tz_param) = value.params.iter().find(|p| p.key == "TZID") {
                if let Some(tzid) = &tz_param.val {
                    return Ok(Self::DateTime(CalendarDateTime::WithTimezone {
                        date_time: naive_dt,
                        tzid: tzid.as_ref().to_string(),
                    }));
                } else {
                    return Err("Found empty TZID param.");
                }
            } else {
                return Ok(Self::DateTime(CalendarDateTime::Floating(naive_dt)));
            };
        };

        Err("Value does not look like a known DATE-TIME")
    }
}

#[cfg(all(test, feature = "parser"))]
mod try_from_tests {
    use super::*;

    #[test]
    fn try_from_utc_dt() {
        let prop = crate::parser::Property {
            name: "TRIGGER".into(),
            val: "20220716T141500Z".into(),
            params: vec![crate::parser::Parameter {
                key: "VALUE".into(),
                val: Some("DATE-TIME".into()),
            }],
        };

        let result = DatePerhapsTime::try_from(&prop);
        let expected = Utc.ymd(2022, 7, 16).and_hms(14, 15, 0);

        assert_eq!(
            result,
            Ok(DatePerhapsTime::DateTime(CalendarDateTime::Utc(expected)))
        );
    }

    #[test]
    fn try_from_naive_date() {
        let prop = crate::parser::Property {
            name: "TRIGGER".into(),
            val: "19970714".into(),
            params: vec![crate::parser::Parameter {
                key: "VALUE".into(),
                val: Some("DATE-TIME".into()),
            }],
        };

        let result = DatePerhapsTime::try_from(&prop);
        let expected = NaiveDate::from_ymd(1997, 7, 14);

        assert_eq!(result, Ok(DatePerhapsTime::Date(expected)));
    }

    #[test]
    fn try_from_dt_with_tz() {
        let prop = crate::parser::Property {
            name: "TRIGGER".into(),
            val: "20220716T141500".into(),
            params: vec![
                crate::parser::Parameter {
                    key: "VALUE".into(),
                    val: Some("DATE-TIME".into()),
                },
                crate::parser::Parameter {
                    key: "TZID".into(),
                    val: Some("MY-TZ-ID".into()),
                },
            ],
        };

        let result = DatePerhapsTime::try_from(&prop);
        let expected = NaiveDate::from_ymd(2022, 7, 16).and_hms(14, 15, 0);

        assert_eq!(
            result,
            Ok(DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone {
                date_time: expected,
                tzid: "MY-TZ-ID".into(),
            }))
        );
    }

    #[test]
    fn try_from_dt_with_empty_tz() {
        let prop = crate::parser::Property {
            name: "TRIGGER".into(),
            val: "20220716T141500".into(),
            params: vec![
                crate::parser::Parameter {
                    key: "VALUE".into(),
                    val: Some("DATE-TIME".into()),
                },
                crate::parser::Parameter {
                    key: "TZID".into(),
                    val: None,
                },
            ],
        };

        let result = DatePerhapsTime::try_from(&prop);

        assert_eq!(result, Err("Found empty TZID param."));
    }

    #[test]
    fn try_from_floating_dt() {
        let prop = crate::parser::Property {
            name: "TRIGGER".into(),
            val: "20220716T141500".into(),
            params: vec![crate::parser::Parameter {
                key: "VALUE".into(),
                val: Some("DATE-TIME".into()),
            }],
        };

        let result = DatePerhapsTime::try_from(&prop);
        let expected = NaiveDate::from_ymd(2022, 7, 16).and_hms(14, 15, 0);

        assert_eq!(
            result,
            Ok(DatePerhapsTime::DateTime(CalendarDateTime::Floating(
                expected
            )))
        );
    }

    #[test]
    fn try_from_non_dt_prop() {
        let prop = crate::parser::Property {
            name: "TZNAME".into(),
            val: "CET".into(),
            params: vec![],
        };

        let result = DatePerhapsTime::try_from(&prop);

        assert_eq!(result, Err("Value does not look like a known DATE-TIME"));
    }
}
