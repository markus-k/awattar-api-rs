use awattar_api::{AwattarZone, PriceData};

#[tokio::main]
async fn main() {
    let prices = PriceData::query_date(AwattarZone::Germany, chrono::Local::today().naive_local())
        .await
        .expect("Querying prices failed.");

    println!("Prices from two days ago to today:");
    for slot in prices.slots_iter() {
        println!(
            "{} - {}: {:.02} €/kWh",
            slot.start(),
            slot.end(),
            slot.price_cents_per_mwh() as f32 / 100_000.00
        );
    }
}
