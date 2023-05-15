// A themometer

fn main() {
    let boiling_water_f: f64 = 212.0;
    let freezeing_water_c: f64 = 0.0;

    let boiling_water_c = (boiling_water_f - 32.0) * (5.0 / 9.0);
    let freezeing_water_f = (freezeing_water_c *(9.0 / 5.0)) + 32.0;

    println!(
        "Water starts boiling at {}C or {}F",
        boiling_water_c, boiling_water_f
    );

    println!(
        "Water starts freezing at {}C or {}F",
        freezeing_water_c, freezeing_water_f
    );
}