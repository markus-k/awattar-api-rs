use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug)]
pub struct PriceSlot {
    /// Start time of this price slot
    pub start: DateTime<Utc>,
    /// End time of this price slot
    pub end: DateTime<Utc>,
    /// Price in Euro-Cents/MWh. The price is stored as an integer to
    /// avoid floating-point errors.
    pub price_cents_per_mwh: i32,
}

impl PriceSlot {
    /// Converts the price to Euro-Cents per kWh
    pub fn price_cents_per_kwh(&self) -> i32 {
        self.price_cents_per_mwh / 1000
    }
}

impl TryFrom<AwattarDataItem> for PriceSlot {
    type Error = AwattarError;

    fn try_from(item: AwattarDataItem) -> Result<Self, Self::Error> {
        let price_cents_mwh = match item.unit.as_str() {
            "Eur/MWh" => Ok((item.marketprice * 100.0) as i32),
            _ => Err(AwattarError::UnsupportedResponse(format!(
                "Unsupported unit {}",
                item.unit
            ))),
        }?;

        Ok(Self {
            start: Utc.timestamp_millis(item.start_timestamp.try_into().map_err(|e| {
                AwattarError::UnsupportedResponse(format!("converting timestamp failed: {e:?}"))
            })?),
            end: Utc.timestamp_millis(item.end_timestamp.try_into().map_err(|e| {
                AwattarError::UnsupportedResponse(format!("converting timestamp failed: {e:?}"))
            })?),
            price_cents_per_mwh: price_cents_mwh,
        })
    }
}

#[derive(Deserialize)]
struct AwattarDataItem {
    start_timestamp: u64,
    end_timestamp: u64,
    marketprice: f32,
    unit: String,
}

#[derive(Deserialize)]
struct AwattarResponse {
    data: Vec<AwattarDataItem>,
}

#[derive(Error, Debug)]
pub enum AwattarError {
    #[error("http request error")]
    Reqwest(#[from] reqwest::Error),
    #[error("api responded with an unsupported response")]
    UnsupportedResponse(String),
}

/// Zone for awattar prices.
#[derive(Debug)]
pub enum AwattarZone {
    Austria,
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
pub async fn query_prices<TZ>(
    zone: AwattarZone,
    start: Option<DateTime<TZ>>,
    end: Option<DateTime<TZ>>,
) -> Result<Vec<PriceSlot>, AwattarError>
where
    TZ: TimeZone,
{
    let client = reqwest::Client::new();
    let query_params = [("start", start), ("end", end)]
        .into_iter()
        .filter_map(|(param, timestamp)| Some((param, timestamp?.timestamp_millis().to_string())))
        .collect::<Vec<_>>();

    let response = client
        .get(zone.api_endpoint())
        .query(&query_params)
        .send()
        .await?
        .json::<AwattarResponse>()
        .await?;

    Ok(response
        .data
        .into_iter()
        .map(|item| PriceSlot::try_from(item))
        .collect::<Result<Vec<_>, _>>()?)
}

/// This is a shortcut for `query_prices::<Utc>(zone, None, None)`.
pub async fn query_prices_now(zone: AwattarZone) -> Result<Vec<PriceSlot>, AwattarError> {
    query_prices::<Utc>(zone, None, None).await
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

        assert_eq!(slot.price_cents_per_mwh, -4209);
    }

    #[test]
    fn test_price_conversion() {
        let slot = PriceSlot {
            start: Utc::now(),
            end: Utc::now(),
            price_cents_per_mwh: 42090,
        };

        assert_eq!(slot.price_cents_per_kwh(), 42);
    }

    #[test]
    fn test_negative_price_conversion() {
        let slot = PriceSlot {
            start: Utc::now(),
            end: Utc::now(),
            price_cents_per_mwh: -42090,
        };

        assert_eq!(slot.price_cents_per_kwh(), -42);
    }
}
