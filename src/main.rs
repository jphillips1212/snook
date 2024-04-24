use reqwest;
use select::document::Document;
use select::predicate::{And, Class, Name, Not, Or};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Create a channel to signal the end of the sleep period
    let (tx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        println!("Ctrl-C pressed, stopping...");
        r.store(false, Ordering::SeqCst);
        tx.send(()).unwrap();  // Send a signal to end the sleep immediately
    }).expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        clear_screen();
        fetch_and_print_matches();

        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                // If we get here because of a signal or the channel is disconnected, break out of the loop
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Continue if the timeout completes without interruption
                continue;
            }
        }
    }

    println!("Exiting cleanly.");
}

fn fetch_and_print_matches() {
    let url = "https://www.snooker.org/res/index.asp?template=21";
    let response = reqwest::blocking::get(url);
    if let Ok(resp) = response {
        if resp.status().is_success() {
            let body = resp.text().unwrap_or_default();
            let document = Document::from(body.as_str());
            if let Some(live_container) = document.find(Class("livecontainer")).next() {
                let mut score_lines = Vec::new();
                for row in live_container.find(Class("gradeA")) {
                    if let Some(score_line) = process_match_row(&row) {
                        score_lines.push(score_line);
                    }
                }
                if !score_lines.is_empty() {
                    print_scores(&score_lines);
                }
            } else {
                println!("No livecontainer found");
            }
        } else {
            println!("Failed to fetch data: {}", resp.status());
        }
    } else {
        println!("Error making request");
    }
}

fn process_match_row(row: &select::node::Node) -> Option<String> {
    let player_elements: Vec<_> = row.find(And(And(Class("player"), Not(Class("h2h"))), Name("td"))).collect();
    let scores: Vec<_> = row.find(Or(Class("first-score"), Class("last-score"))).map(|s| s.text().trim().to_string()).collect();

    if player_elements.len() == 2 && scores.len() == 2 {
        let player1_name = extract_player_name(&player_elements[0]);
        let player2_name = extract_player_name(&player_elements[1]);
        let score1 = &scores[0];
        let score2 = &scores[1];
        Some(format!("{} {} - {} {}", player1_name, score1, score2, player2_name))
    } else {
        None
    }
}

fn extract_player_name(player_element: &select::node::Node) -> String {
    player_element.find(Name("a")).next()
        .map(|n| n.text())
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|| "Unknown Player".to_string())
}

fn print_scores(scores: &[String]) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let max_score_width = scores.iter().map(|s| s.len()).max().unwrap_or(0);
    let total_padding = (width as usize - max_score_width) / 2;
    let border = "*".repeat(max_score_width);

    println!("{:padding$}+{}+", "", border, padding = total_padding);
    for score in scores {
        let spaces_needed = max_score_width - score.len();
        let left_padding = spaces_needed / 2;
        let right_padding = spaces_needed - left_padding;
        let formatted_line = format!("{:left$}{}{:right$}", "", score, "", left = left_padding, right = right_padding);
        println!("{:padding$}|{}|", "", formatted_line, padding = total_padding);
    }
    println!("{:padding$}+{}+", "", border, padding = total_padding);
}


fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}
