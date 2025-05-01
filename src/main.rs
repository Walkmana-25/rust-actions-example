mod args;
mod utils;
mod weather;

use anyhow::Result;
use chrono::{Duration, Local, NaiveDate, TimeZone};
use clap::Parser;
use reqwest::Client;

use args::Args;
use utils::calculate_average;
use weather::fetch_weather_data;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = Client::new();

    println!(
        "Fetching weather data for latitude: {}, longitude: {}...",
        args.latitude, args.longitude
    );

    let weather_data = fetch_weather_data(&client, args.latitude, args.longitude).await?;

    let now = Local::now().date_naive();
    let yesterday = now - Duration::days(1);

    let mut today_temps = Vec::new();
    let mut yesterday_temps = Vec::new();

    if weather_data.hourly.time.len() != weather_data.hourly.temperature_2m.len() {
        anyhow::bail!("API response has mismatched time and temperature data lengths.");
    }

    // Populate today's and yesterday's temperatures for average calculation
    for (i, time_str) in weather_data.hourly.time.iter().enumerate() {
        // Only parse the date part for average calculation logic
        if let Ok(date) = NaiveDate::parse_from_str(&time_str[0..10], "%Y-%m-%d") {
            if date == now {
                today_temps.push(weather_data.hourly.temperature_2m[i]);
            } else if date == yesterday {
                yesterday_temps.push(weather_data.hourly.temperature_2m[i]);
            }
        } else {
            // Keep the warning for the date parsing specific to average calculation
            eprintln!(
                "Warning: Could not parse date for average calculation from timestamp: {}",
                time_str
            );
        }
    }

    println!("\n--- Hourly Weather Data (Every 6 Hours) ---");
    // Print hourly data for yesterday and today only, every 6 hours
    for (i, (time_str, temp)) in weather_data
        .hourly
        .time
        .iter()
        .zip(weather_data.hourly.temperature_2m.iter())
        .enumerate()
    {
        // Only process every 6th hour
        if i % 6 != 0 {
            continue;
        }

        // Attempt to parse the full datetime string for hourly display
        if let Ok(naive_datetime) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M") {
            // Assume local timezone if parsing NaiveDateTime succeeds
            match Local.from_local_datetime(&naive_datetime) {
                chrono::LocalResult::Single(local_datetime) => {
                    let date = local_datetime.date_naive();
                    if date == now || date == yesterday {
                        let display_time = local_datetime.format("%Y-%m-%d %H:%M");
                        println!("{}: {:.1}°C", display_time, temp);
                    }
                }
                chrono::LocalResult::Ambiguous(_, _) => {
                    let date = naive_datetime.date();
                    if date == now || date == yesterday {
                        println!("{}: {:.1}°C (Ambiguous Local Time)", time_str, temp);
                    }
                }
                chrono::LocalResult::None => {
                    let date = naive_datetime.date();
                    if date == now || date == yesterday {
                        println!("{}: {:.1}°C (Invalid Local Time)", time_str, temp);
                    }
                }
            }
        } else if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&(time_str.replace("Z", "+00:00"))) {
            // Fallback to RFC3339 parsing if NaiveDateTime fails
            let date = datetime.date_naive();
            if date == now || date == yesterday {
                let display_time = datetime.with_timezone(&Local).format("%Y-%m-%d %H:%M");
                println!("{}: {:.1}°C", display_time, temp);
            }
        } else {
            // If all parsing fails, print a warning but only for yesterday and today
            if let Ok(date) = NaiveDate::parse_from_str(&time_str[0..10], "%Y-%m-%d") {
                if date == now || date == yesterday {
                    eprintln!(
                        "Warning: Could not parse timestamp for hourly display: {}",
                        time_str
                    );
                    println!("{}: {:.1}°C", time_str, temp);
                }
            }
        }
    }

    println!("\n--- Average Temperatures ---"); // Changed header for clarity

    match calculate_average(&today_temps) {
        Some(avg) => println!("Today's average temperature: {:.2}°C", avg),
        None => println!("Could not calculate today's average temperature (no data)."),
    }

    match calculate_average(&yesterday_temps) {
        Some(avg) => println!("Yesterday's average temperature: {:.2}°C", avg),
        None => println!("Could not calculate yesterday's average temperature (no data)."),
    }

    Ok(())
}
