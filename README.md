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
// get prices for the last two days
let prices = query_prices(
    AwattarZone::Germany,
    Some(chrono::Local::now() - chrono::Duration::days(2)),
    Some(chrono::Local::now()),
).await.unwrap();

for slot in prices {
    println!(
        "{} - {}: {:.02} €/kWh",
        slot.start,
        slot.end,
        slot.price_cents_per_kwh() as f32 / 100.00
    );
}
```

## License

This crate is licensed under the MIT license.
