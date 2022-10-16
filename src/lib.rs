#![doc = include_str!("../README.md")]
#![warn(rust_2018_idioms)]

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use thiserror::Error;

/// A single price slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceSlot {
    /// Start time of this price slot
    start: DateTime<Utc>,
    /// End time of this price slot
    end: DateTime<Utc>,
    /// Price in Euro-Cents/MWh. The price is stored as an integer to
    /// avoid floating-point errors.
    price_cents_per_mwh: i32,
}

impl PriceSlot {
    /// DateTime this `PriceSlot` is valid from.
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Non-inclusive DateTime this `PriceSlot` is valid to.
    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }

    /// Price in Euro-Cents/MWh.
    pub fn price_cents_per_mwh(&self) -> i32 {
        self.price_cents_per_mwh
    }
}

impl TryFrom<AwattarDataItem> for PriceSlot {
    type Error = AwattarError;

    fn try_from(item: AwattarDataItem) -> Result<Self, Self::Error> {
        let price_cents_per_mwh = match item.unit.as_str() {
            "Eur/MWh" => Ok((item.marketprice * 100.0) as i32),
            _ => Err(AwattarError::UnsupportedResponse(format!(
                "Unsupported unit {}",
                item.unit
            ))),
        }?;

        Ok(Self {
            start: Utc.timestamp_millis(item.start_timestamp),
            end: Utc.timestamp_millis(item.end_timestamp),
            price_cents_per_mwh,
        })
    }
}

/// Holds a set of price slots and provides some utility functions for working with price data.
#[derive(Clone, Debug)]
pub struct PriceData {
    slots: Vec<PriceSlot>,
    zone: AwattarZone,
}

impl PriceData {
    /// Query prices from the awattar API between the given start- and end-datetime in the given
    /// zone.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
    /// use awattar_api::{AwattarZone, PriceData};
    /// use chrono::{Local, TimeZone};
    ///
    /// let prices = PriceData::query(
    ///         AwattarZone::Germany,
    ///         Some(Local.ymd(2022, 08, 1).and_hms(0, 0, 0)),
    ///         Some(Local.ymd(2022, 08, 2).and_hms(0, 0, 0)),
    ///     )
    ///         .await
    ///         .unwrap();
    /// println!("Prices: {:?}", prices);
    /// # });
    /// ```
    pub async fn query<TZ: TimeZone>(
        zone: AwattarZone,
        start: Option<DateTime<TZ>>,
        end: Option<DateTime<TZ>>,
    ) -> Result<Self, AwattarError> {
        let client = reqwest::Client::new();
        let query_params = [("start", start), ("end", end)]
            .into_iter()
            .filter_map(|(param, timestamp)| {
                Some((param, timestamp?.timestamp_millis().to_string()))
            })
            .collect::<Vec<_>>();

        let response = client
            .get(zone.api_endpoint())
            .query(&query_params)
            .send()
            .await?
            .json::<AwattarResponse>()
            .await?;

        Ok(Self::from_slots(
            response
                .data
                .into_iter()
                .map(PriceSlot::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            zone,
        ))
    }

    /// Query prices from the awattar API for a given date.
    ///
    /// The NaiveDate is converted to a timezone-aware Date using the given [`AwattarZone`]s local
    /// timezone. This always yields price data from 00:00 on the start date to 00:00 on the
    /// end-date (24 slots on days without switch between daylight saving and standard time).
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
    /// use awattar_api::{AwattarZone, PriceData};
    /// use chrono::Local;
    ///
    /// let prices = PriceData::query_date(AwattarZone::Germany, Local::today().naive_local())
    ///     .await
    ///     .unwrap();
    /// println!("Prices: {:?}", prices);
    /// # });
    /// ```
    pub async fn query_date(zone: AwattarZone, date: NaiveDate) -> Result<Self, AwattarError> {
        let start = date
            .and_hms(0, 0, 0)
            .and_local_timezone(zone.timezone())
            .unwrap();
        let end = (date + Duration::days(1))
            .and_hms(0, 0, 0)
            .and_local_timezone(zone.timezone())
            .unwrap();

        Self::query(zone, Some(start), Some(end)).await
    }

    /// Create a new instance from a [`Vec`] of [`PriceSlot`]s.
    pub fn from_slots(slots: Vec<PriceSlot>, zone: AwattarZone) -> Self {
        Self { slots, zone }
    }

    /// Returns the number of slots this instance is holding.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Return `true` when this instance contains any [`PriceSlot`]s.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Return a [`Vec`] of [`PriceSlot`]s this instance is holding.
    pub fn slots(&self) -> &Vec<PriceSlot> {
        &self.slots
    }

    /// Currently only used for deprecated API, but could be turned into a public
    /// API if there is any need for it.
    fn into_slots(self) -> Vec<PriceSlot> {
        self.slots
    }

    /// Provides an iterator over all [`PriceSlot`]s this instance holds. Useful for things like
    /// calculating the average price over a day.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
    /// use awattar_api::{AwattarZone, PriceData};
    /// use chrono::Local;
    ///
    /// let prices = PriceData::query_date(AwattarZone::Germany, Local::today().naive_local())
    ///     .await
    ///     .unwrap();
    /// let avg_price = prices.slots_iter().fold(0, |sum, slot| sum + slot.price_cents_per_mwh())
    ///     / prices.len() as i32;
    /// # });
    /// ```
    pub fn slots_iter(&self) -> impl Iterator<Item = &PriceSlot> {
        self.slots.iter()
    }

    /// Finds and returns the [`PriceSlot`] for the given datetime.
    ///
    /// If no slot could be found, `None` is returned.
    pub fn slot_for_datetime<TZ: TimeZone>(&self, datetime: DateTime<TZ>) -> Option<&PriceSlot> {
        self.slots
            .iter()
            .find(|slot| slot.start() >= datetime && slot.end < datetime)
    }

    /// Returns the [`PriceSlot`] with the lowest price.
    ///
    /// If this instance does not contain any price slots, `None` is returned.
    pub fn min_price(&self) -> Option<&PriceSlot> {
        self.slots
            .iter()
            .min_by_key(|slot| slot.price_cents_per_mwh())
    }

    /// Returns the [`PriceSlot`] with the highest price.
    ///
    /// If this instance does not contain any price slots, `None` is returned.
    pub fn max_price(&self) -> Option<&PriceSlot> {
        self.slots
            .iter()
            .max_by_key(|slot| slot.price_cents_per_mwh())
    }

    /// Returns the zone this instance belongs in.
    pub fn zone(&self) -> AwattarZone {
        self.zone
    }
}

/// Struct for deserialzing time-slots from the awattar API.
#[derive(Deserialize)]
struct AwattarDataItem {
    start_timestamp: i64,
    end_timestamp: i64,
    marketprice: f32,
    unit: String,
}

/// Struct for deserializing the JSON response from the awattar API.
#[derive(Deserialize)]
struct AwattarResponse {
    data: Vec<AwattarDataItem>,
}

/// Common error enum for this crate.
#[derive(Error, Debug)]
pub enum AwattarError {
    #[error("HTTP request error")]
    Reqwest(#[from] reqwest::Error),
    #[error("API responded with an unsupported response")]
    UnsupportedResponse(String),
}

/// Zone for awattar prices.
///
/// Currently supports Austria and Germany, but could expand in the future as Germany might
/// split their price zones or awattar adds support for further countries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwattarZone {
    /// Prices for Austria
    Austria,
    /// Prices for Germany
    Germany,
}

impl AwattarZone {
    /// Returns the API endpoint for the given zone.
    pub const fn api_endpoint(&self) -> &'static str {
        match self {
            AwattarZone::Austria => "https://api.awattar.at/v1/marketdata",
            AwattarZone::Germany => "https://api.awattar.de/v1/marketdata",
        }
    }

    /// Returns the `TimeZone` of the [`AwattarZone`].
    ///
    /// While all currently support zones have the same TZ, it's not unheard of that a
    /// country might eliminate DST or future zones may have different timezones.
    ///
    /// This especially comes in handy when you want to query times from the first to the
    /// last hour of a day (i.e. full 24 hours).
    pub const fn timezone(&self) -> chrono_tz::Tz {
        match self {
            AwattarZone::Austria => chrono_tz::Europe::Vienna,
            AwattarZone::Germany => chrono_tz::Europe::Berlin,
        }
    }
}

/// Query prices from the API in the given `zone` with an optional `start` and `end`
/// DateTime.
///
/// Supplying only `start` returns only prices from `start` up until `start` + 24 hours.
/// Supplying `start` and `end` returns all prices within the given datetimes (within the
/// limits of the API).
/// Supplying neither `start` nor `end` returns all prices starting now, up to 24 hours
/// into the future. Better use `query_prices_now()` as a convencience function in this
/// case.
#[deprecated(since = "0.2.0", note = "Use `PriceData::query` instead")]
pub async fn query_prices<TZ>(
    zone: AwattarZone,
    start: Option<DateTime<TZ>>,
    end: Option<DateTime<TZ>>,
) -> Result<Vec<PriceSlot>, AwattarError>
where
    TZ: TimeZone,
{
    Ok(PriceData::query(zone, start, end).await?.into_slots())
}

/// This is a shortcut for `query_prices::<Utc>(zone, None, None)`.
#[deprecated(since = "0.2.0", note = "Use `PriceData::query` instead")]
pub async fn query_prices_now(zone: AwattarZone) -> Result<Vec<PriceSlot>, AwattarError> {
    Ok(PriceData::query::<Utc>(zone, None, None)
        .await?
        .into_slots())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priceslot_from_item() {
        let item = AwattarDataItem {
            start_timestamp: 1428591600000,
            end_timestamp: 1428595200000,
            marketprice: 42.09,
            unit: "Eur/MWh".to_owned(),
        };

        let slot = PriceSlot::try_from(item).unwrap();

        assert_eq!(slot.start, Utc.timestamp_millis(1428591600000));
        assert_eq!(slot.end, Utc.timestamp_millis(1428595200000));
        assert_eq!(slot.price_cents_per_mwh, 4209);
    }

    #[test]
    fn test_priceslot_from_item_negative() {
        let item = AwattarDataItem {
            start_timestamp: 0,
            end_timestamp: 0,
            marketprice: -42.09,
            unit: "Eur/MWh".to_owned(),
        };

        let slot = PriceSlot::try_from(item).unwrap();

        assert_eq!(slot.price_cents_per_mwh(), -4209);
    }
}
