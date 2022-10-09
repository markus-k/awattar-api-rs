# `awattar-api` – Rust client for the awattar price API

This crates is an API client for the [awattar](https://www.awattar.de) price API.

The API of this crate is currently not considered stable and is likely to change
in future releases.

## Usage

For a full example have a look at the `examples/`-directory.

Add `awattar-api` to you dependencies:
``` toml
[dependencies]
awattar-api = "0.1.0"
```

Querying prices is simple:
``` rust
use awattar_api::*;
use chrono::Utc;

#[tokio::main]
async fn main() {
    let date = Utc::today().naive_local();

    let prices = PriceData::query_date(AwattarZone::Germany, date)
        .await
        .unwrap();

    for slot in prices.slots_iter() {
        println!(
            "{} - {}: {:.02} €/kWh",
            slot.start(),
            slot.end(),
            slot.price_cents_per_mwh() as f32 / 100_000.00
        );
    }
}
```

## License

This crate is licensed under the MIT license.
