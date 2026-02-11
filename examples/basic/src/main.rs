use std::io::{self, Write};

use basic::{ConvexApi, GamesTable};
use convex::ConvexClient;
use rand::Rng;

const CONVEX_URL: &str = "https://notable-orca-705.convex.cloud";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut client = ConvexClient::new(CONVEX_URL).await?;

    // =========================================================================
    // Typed query: getGame returns Option<GamesTable> directly
    // =========================================================================
    let game: Option<GamesTable> = client.query_games_get_game().await?;

    let (wins, losses) = match &game {
        Some(g) => (g.win_count as i32, g.loss_count as i32),
        None => (0, 0),
    };

    println!("Welcome to the Number Guessing Game!");
    println!("Current record - Wins: {}, Losses: {}", wins, losses);
    println!("I'm thinking of a number between 1 and 100.");

    let secret_number = rand::thread_rng().gen_range(1..=100);
    let mut attempts = 0;
    const MAX_ATTEMPTS: i32 = 10;

    loop {
        print!("Enter your guess (1-100): ");
        io::stdout().flush()?;

        let mut guess = String::new();
        io::stdin().read_line(&mut guess)?;

        let guess: i32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Please enter a valid number!");
                continue;
            }
        };

        attempts += 1;

        match guess.cmp(&secret_number) {
            std::cmp::Ordering::Less => println!("Too low!"),
            std::cmp::Ordering::Greater => println!("Too high!"),
            std::cmp::Ordering::Equal => {
                println!("Congratulations! You won in {} attempts!", attempts);
                client.games_win_game().await?;
                println!("Win saved!");
                break;
            }
        }

        if attempts >= MAX_ATTEMPTS {
            println!(
                "Sorry, you've run out of attempts! The number was {}",
                secret_number
            );
            client.games_loss_game().await?;
            break;
        }

        println!("You have {} attempts remaining.", MAX_ATTEMPTS - attempts);
    }

    // Wait a moment for the mutation to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // =========================================================================
    // Typed query: listGames returns Vec<GamesTable> directly
    // =========================================================================
    let all_games: Vec<GamesTable> = client.query_games_list_games().await?;
    println!("\nTotal game records: {}", all_games.len());

    if let Some(g) = all_games.first() {
        println!(
            "Latest - Wins: {}, Losses: {}, Status: {:?}",
            g.win_count as i32, g.loss_count as i32, g.status
        );
    }

    Ok(())
}
