use crate::events::Event;
use rusqlite::{params, Connection, OpenFlags};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::Sender;

pub fn init_database(tx: Sender<Event>) {
    std::thread::spawn(move || {
        let db_path = "opensky_aircraft.db";
        let csv_path = "data/aircraft-database-complete-2025-08.csv";

        let file = match File::open(csv_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = tx.send(Event::DbError(format!("Missing CSV: {}", e)));
                return;
            }
        };

        let total_size = file.metadata().unwrap().len() as f32;
        let mut bytes_processed = 0;
        let conn = Connection::open(db_path).unwrap();

        // Create Schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS aircraft (
                icao24 TEXT PRIMARY KEY,
                manufacturerName TEXT,
                model TEXT,
                operator TEXT,
                operatorCallsign TEXT,
                owner TEXT,
                registration TEXT,
                typecode TEXT
            )",
            [],
        )
        .unwrap();

        // Map Headers
        let mut rdr = csv::ReaderBuilder::new()
            .quote(b'\'')
            .has_headers(true)
            .from_reader(BufReader::new(file));

        let headers = match rdr.headers() {
            Ok(h) => h.clone(),
            Err(e) => {
                let _ = tx.send(Event::DbError(format!("Header Error: {}", e)));
                return;
            }
        };
        let find_col = |name: &str| {
            headers.iter().position(|h| {
                let clean_h = h.trim_start_matches('\u{feff}').trim().to_lowercase();
                clean_h == name.to_lowercase()
            })
        };

        let idx_icao = match find_col("icao24") {
            Some(i) => i,
            None => {
                let _ = tx.send(Event::DbError(format!(
                    "CSV Error: Could not find 'icao24' column. Found: {:?}",
                    headers
                )));
                return; // Exit thread gracefully instead of panicking
            }
        };
        let idx_mfr = find_col("manufacturername");
        let idx_mod = find_col("model");
        let idx_oper = find_col("operator");
        let idx_call = find_col("operatorcallsign");
        let idx_own = find_col("owner");
        let idx_reg = find_col("registration");
        let idx_type = find_col("typecode");

        // Bulk Insert
        let db_tx = conn.unchecked_transaction().unwrap();
        {
            let mut stmt = db_tx
                .prepare("INSERT OR REPLACE INTO aircraft VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .unwrap();

            for (i, result) in rdr.records().enumerate() {
                let record = result.unwrap();
                bytes_processed += record.as_slice().len();

                let clean = |idx: Option<usize>| {
                    idx.and_then(|i| record.get(i))
                        .map(|s| s.trim_matches(|c| c == '\'' || c == '"').trim())
                        .unwrap_or("")
                        .to_string()
                };

                let _ = stmt.execute(params![
                    clean(Some(idx_icao)).to_lowercase(),
                    clean(idx_mfr),
                    clean(idx_mod),
                    clean(idx_oper),
                    clean(idx_call),
                    clean(idx_own),
                    clean(idx_reg),
                    clean(idx_type),
                ]);

                if i % 2000 == 0 {
                    let _ = tx.send(Event::DbProgress(bytes_processed as f32 / total_size));
                }
            }
        }
        db_tx.commit().unwrap();
        let _ = tx.send(Event::DbDone);
    });
}

pub fn decorate_flights(mut flights: Vec<crate::models::Flight>) -> Vec<crate::models::Flight> {
    let db_path = "opensky_aircraft.db";

    // If DB doesn't exist yet, just return the raw flights
    if !std::path::Path::new(db_path).exists() {
        return flights;
    }

    let conn = match Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(_) => return flights,
    };

    let mut stmt = conn
        .prepare(
            "SELECT manufacturerName, model, operator, operatorCallsign, registration, typecode 
        FROM aircraft WHERE icao24 = ?",
        )
        .unwrap();

    for flight in &mut flights {
        let icao = flight.icao24.trim().to_lowercase();

        let details = stmt.query_row([&icao], |row| {
            Ok((
                row.get::<_, String>(0)?, // manufacturer
                row.get::<_, String>(1)?, // model
                row.get::<_, String>(2)?, // operator
                row.get::<_, String>(3)?, // callsign
                row.get::<_, String>(4)?, // registration
                row.get::<_, String>(5)?, // typecode
            ))
        });

        if let Ok((mfr, md, op, call, reg, ty)) = details {
            flight.manufacturer = Some(mfr);
            flight.model = Some(md);
            flight.operator = Some(op);
            flight.operator_callsign = Some(call);
            flight.registration = Some(reg);
            flight.aircraft_type = Some(ty);
        }
    }
    flights
}
