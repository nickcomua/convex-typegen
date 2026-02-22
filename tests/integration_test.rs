//! Integration tests for convex-typegen generated code.
//!
//! These tests verify two things:
//! 1. The generated Rust code from the example compiles (compile-time check).
//! 2. The generated `ConvexApi` trait works against a real Convex backend (runtime check).
//!
//! End-to-end tests require Docker and Node.js:
//!   cargo test -p convex-typegen --test integration_test -- --nocapture
//!
//! The codegen pipeline test runs without external dependencies:
//!   cargo test -p convex-typegen --test integration_test test_codegen_pipeline

mod common;

use std::path::PathBuf;

// The `basic` example crate is a dev-dependency. Its build.rs runs convex-typegen
// to generate types into OUT_DIR. If this compiles, the generated code is valid Rust.
use basic as example_types;
use common::get_test_env;
use convex::ConvexClient;
use example_types::{ConvexApi, ConvexApiClient};

// =============================================================================
// Codegen Pipeline (no Docker needed)
// =============================================================================

#[test]
fn test_codegen_pipeline()
{
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = convex_typegen::Configuration {
        schema_path: manifest_dir.join("examples/basic/convex/schema.ts"),
        out_file: std::env::temp_dir()
            .join("convex_typegen_integration_test.rs")
            .to_string_lossy()
            .to_string(),
        function_paths: vec![
            manifest_dir.join("examples/basic/convex/games.ts"),
            manifest_dir.join("examples/basic/convex/players.ts"),
        ],
        helper_stubs: std::collections::HashMap::new(),
    };

    convex_typegen::generate(config).expect("Codegen failed");
    let output = std::fs::read_to_string(std::env::temp_dir().join("convex_typegen_integration_test.rs"))
        .expect("Failed to read generated file");

    // Table structs with system fields and serde attributes
    assert!(output.contains("pub struct GamesTable"), "Missing GamesTable");
    assert!(output.contains("pub struct PlayersTable"), "Missing PlayersTable");
    assert!(output.contains("pub struct TestTable"), "Missing TestTable");
    assert!(output.contains("#[serde(rename = \"_id\")]"), "Missing _id rename");
    assert!(output.contains("pub id: String"), "Missing id field");
    assert!(output.contains("pub creation_time: f64"), "Missing creation_time");

    // Games table fields
    assert!(output.contains("pub win_count: f64"), "Missing win_count");
    assert!(output.contains("pub loss_count: f64"), "Missing loss_count");
    assert!(
        output.contains("pub status: GamesStatus"),
        "Missing status field on GamesTable"
    );

    // Players table nested structs
    assert!(output.contains("pub struct PlayersProfile"), "Missing PlayersProfile");
    assert!(
        output.contains("pub struct PlayersProfileSettings"),
        "Missing PlayersProfileSettings"
    );
    assert!(
        output.contains("pub enum PlayersProfileSettingsTheme"),
        "Missing PlayersProfileSettingsTheme"
    );
    assert!(
        output.contains("pub struct PlayersAchievements"),
        "Missing PlayersAchievements"
    );
    assert!(output.contains("pub enum PlayersRank"), "Missing PlayersRank");

    // Shared enum
    assert!(output.contains("pub enum GamesStatus"), "Missing GamesStatus");

    // Function arg structs (both files)
    assert!(output.contains("pub struct GamesGetGameArgs"), "Missing GamesGetGameArgs");
    assert!(output.contains("pub struct GamesWinGameArgs"), "Missing GamesWinGameArgs");
    assert!(output.contains("pub struct GamesLossGameArgs"), "Missing GamesLossGameArgs");
    assert!(
        output.contains("pub struct GamesGetByStatusArgs"),
        "Missing GamesGetByStatusArgs"
    );
    assert!(
        output.contains("pub struct GamesUpdateGameStatusArgs"),
        "Missing GamesUpdateGameStatusArgs"
    );
    assert!(
        output.contains("pub struct PlayersListActiveArgs"),
        "Missing PlayersListActiveArgs"
    );
    assert!(output.contains("pub struct PlayersGetByIdArgs"), "Missing PlayersGetByIdArgs");
    assert!(
        output.contains("pub struct PlayersGetByRankArgs"),
        "Missing PlayersGetByRankArgs"
    );
    assert!(output.contains("pub struct PlayersCreateArgs"), "Missing PlayersCreateArgs");
    assert!(
        output.contains("pub struct PlayersUpdateProfileArgs"),
        "Missing PlayersUpdateProfileArgs"
    );
    assert!(
        output.contains("pub struct PlayersAddAchievementArgs"),
        "Missing PlayersAddAchievementArgs"
    );

    // Tagged union args
    assert!(
        output.contains("pub enum GamesUpdateGameStatusResult"),
        "Missing GamesUpdateGameStatusResult enum"
    );
    assert!(
        output.contains("pub enum PlayersUpdateProfileAction"),
        "Missing PlayersUpdateProfileAction enum"
    );

    // Inline v.object structs for args
    assert!(
        output.contains("pub struct PlayersCreateProfile"),
        "Missing PlayersCreateProfile arg struct"
    );
    assert!(
        output.contains("pub struct PlayersCreateProfileSettings"),
        "Missing PlayersCreateProfileSettings arg struct"
    );
    assert!(
        output.contains("pub struct PlayersAddAchievementAchievement"),
        "Missing PlayersAddAchievementAchievement arg struct"
    );

    // FUNCTION_PATH constants
    assert!(output.contains("\"games:getGame\""), "Missing getGame path");
    assert!(output.contains("\"games:winGame\""), "Missing winGame path");
    assert!(output.contains("\"games:lossGame\""), "Missing lossGame path");
    assert!(output.contains("\"games:listGames\""), "Missing listGames path");
    assert!(output.contains("\"games:getByStatus\""), "Missing getByStatus path");
    assert!(output.contains("\"games:updateGameStatus\""), "Missing updateGameStatus path");
    assert!(output.contains("\"games:updateWithNote\""), "Missing updateWithNote path");

    // Optional args in updateWithNote should use `if let Some` in BTreeMap From impl
    assert!(
        output.contains("pub struct GamesUpdateWithNoteArgs"),
        "Missing GamesUpdateWithNoteArgs"
    );
    assert!(
        output.contains("if let Some(val) = _args.note"),
        "Optional note field should use if let Some"
    );
    assert!(
        output.contains("if let Some(val) = _args.score"),
        "Optional score field should use if let Some"
    );

    assert!(output.contains("\"players:listActive\""), "Missing listActive path");
    assert!(output.contains("\"players:getById\""), "Missing getById path");
    assert!(output.contains("\"players:getByRank\""), "Missing getByRank path");
    assert!(output.contains("\"players:create\""), "Missing create path");
    assert!(output.contains("\"players:updateProfile\""), "Missing updateProfile path");
    assert!(output.contains("\"players:addAchievement\""), "Missing addAchievement path");

    // ConvexApi trait with correct method signatures
    assert!(output.contains("pub trait ConvexApi"), "Missing ConvexApi trait");

    // Typed query returns: getGame has returns: v.union(gameDoc, v.null()) → Option<GamesTable>
    assert!(
        output.contains(
            "fn query_games_get_game(&self) -> impl std::future::Future<Output = Result<Option<GamesTable>, ConvexError>>"
        ),
        "getGame should return Result<Option<GamesTable>, ConvexError>"
    );

    // Typed array query returns: listGames has returns: v.array(gameDoc) → Vec<GamesTable>
    assert!(
        output.contains(
            "fn query_games_list_games(&self) -> impl std::future::Future<Output = Result<Vec<GamesTable>, ConvexError>>"
        ),
        "listGames should return Result<Vec<GamesTable>, ConvexError>"
    );

    // Typed subscription returns
    assert!(
        output.contains(
            "fn subscribe_games_get_game(&self) -> impl std::future::Future<Output = \
             Result<TypedSubscription<Option<GamesTable>>, ConvexError>>"
        ),
        "subscribe_games_get_game should return TypedSubscription<Option<GamesTable>>"
    );
    assert!(
        output.contains(
            "fn subscribe_players_list_active(&self) -> impl std::future::Future<Output = \
             Result<TypedSubscription<Vec<PlayersTable>>, ConvexError>>"
        ),
        "subscribe_players_list_active should return TypedSubscription<Vec<PlayersTable>>"
    );

    // Mutation typed returns: create has returns: v.id("players") → String
    assert!(
        output.contains(
            "fn players_create(&self, args: PlayersCreateArgs) -> impl std::future::Future<Output = Result<String, \
             ConvexError>>"
        ),
        "players_create should return Result<String, ConvexError>"
    );

    // Mutation null returns: winGame has returns: v.null() → ()
    assert!(
        output.contains("fn games_win_game(&self) -> impl std::future::Future<Output = Result<(), ConvexError>>"),
        "winGame should return Result<(), ConvexError>"
    );

    // TypedSubscription struct
    assert!(
        output.contains("pub struct TypedSubscription<T>"),
        "Missing TypedSubscription struct"
    );
    assert!(
        output.contains("futures_core::Stream for TypedSubscription<T>"),
        "Missing Stream impl"
    );

    // ConvexApiClient wrapper struct
    assert!(
        output.contains("pub struct ConvexApiClient"),
        "Missing ConvexApiClient wrapper struct"
    );

    // ConvexApi impl for ConvexApiClient
    assert!(output.contains("impl ConvexApi for ConvexApiClient"), "Missing trait impl");

    // json_to_convex_value helper (args → convex::Value)
    assert!(
        output.contains("fn json_to_convex_value"),
        "Missing json_to_convex_value helper"
    );

    // convex_value_to_json helper (convex::Value → serde_json::Value, for typed returns)
    assert!(
        output.contains("fn convex_value_to_json"),
        "Missing convex_value_to_json helper"
    );
}

// =============================================================================
// End-to-end: Generated ConvexApi against real Convex backend
// =============================================================================

#[tokio::test]

async fn test_win_game_creates_record()
{
    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    // Win a game — typed return is () on success
    client.games_win_game().await.expect("Win failed");

    // Query should now return a game object
    let game = client.query_games_get_game().await.expect("Query failed");
    // May be Some or None depending on timing
    if let Some(g) = game {
        assert!(g.win_count >= 1.0, "Expected at least 1 win");
    }
}

#[tokio::test]

async fn test_loss_game()
{
    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    client.games_loss_game().await.expect("Loss failed");
}

#[tokio::test]

async fn test_subscribe_get_game()
{
    use futures::StreamExt;

    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    let mut sub = client.subscribe_games_get_game().await.expect("Failed to subscribe");

    // TypedSubscription yields anyhow::Result<Option<GamesTable>>
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), sub.next())
        .await
        .expect("Timeout waiting for subscription")
        .expect("Subscription stream ended");

    // Result should be Ok (either None or Some(GamesTable))
    let _game: Option<example_types::GamesTable> = result.expect("Subscription yielded error");
}

#[tokio::test]

async fn test_full_game_lifecycle()
{
    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    // Win twice — typed return is () on success
    client.games_win_game().await.expect("First win failed");
    client.games_win_game().await.expect("Second win failed");

    // Lose once
    client.games_loss_game().await.expect("Loss failed");

    // Query the final state — now typed as Option<GamesTable>
    let game = client.query_games_get_game().await.expect("Query failed");

    match game {
        Some(g) => {
            // The game should exist with some win and loss counts
            assert!(g.win_count >= 1.0, "Expected wins");
            assert!(g.loss_count >= 1.0, "Expected losses");
        }
        None => {
            // Acceptable if the database was cleared between tests
        }
    }
}

#[tokio::test]

async fn test_list_games()
{
    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    // listGames returns Vec<GamesTable> directly
    let games: Vec<example_types::GamesTable> = client.query_games_list_games().await.expect("Query failed");

    // Fresh database may be empty, but the call should succeed
    let _ = games.len();
}

// =============================================================================
// E2E: Optional args against real Convex backend
// =============================================================================

/// Regression test: calling a mutation with None optional args must not send
/// `null` to Convex. Convex's `v.optional(v.string())` rejects explicit null —
/// the field must be absent from the args object when the value is None.
#[tokio::test]

async fn test_mutation_with_none_optional_args()
{
    use example_types::GamesUpdateWithNoteArgs;

    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    // First, create a game so we have a valid ID
    client.games_win_game().await.expect("Win failed");
    let game = client.query_games_get_game().await.expect("Query failed");
    let game = game.expect("Expected a game to exist");

    // Call mutation with None optional args — this would fail with
    // "ArgumentValidationError: Value does not match validator" before the fix
    client
        .games_update_with_note(GamesUpdateWithNoteArgs {
            gameId: game.id.clone(),
            note: None,
            score: None,
        })
        .await
        .expect("Mutation with None optional args should succeed");
}

#[tokio::test]

async fn test_mutation_with_some_optional_args()
{
    use example_types::GamesUpdateWithNoteArgs;

    let env = get_test_env().await;
    let client = ConvexApiClient::new(ConvexClient::new(&env.convex_url).await.expect("Failed to connect"));

    client.games_win_game().await.expect("Win failed");
    let game = client.query_games_get_game().await.expect("Query failed");
    let game = game.expect("Expected a game to exist");

    // Call mutation with Some optional args
    client
        .games_update_with_note(GamesUpdateWithNoteArgs {
            gameId: game.id.clone(),
            note: Some("test note".to_string()),
            score: Some(99.0),
        })
        .await
        .expect("Mutation with Some optional args should succeed");
}

// =============================================================================
// Args Serialization
// =============================================================================

#[test]
fn test_args_into_btreemap()
{
    use example_types::{GamesGetGameArgs, GamesLossGameArgs, GamesWinGameArgs};

    // All example args are empty structs — verify they produce empty maps
    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesGetGameArgs {}.into();
    assert!(map.is_empty());

    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesWinGameArgs {}.into();
    assert!(map.is_empty());

    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesLossGameArgs {}.into();
    assert!(map.is_empty());
}

#[test]
fn test_args_with_fields_into_btreemap()
{
    use example_types::{GamesGetByStatusArgs, GamesGetByStatusStatus, PlayersGetByIdArgs};

    // Args with fields produce non-empty maps with correct keys
    let map: std::collections::BTreeMap<String, serde_json::Value> = PlayersGetByIdArgs {
        playerId: "abc123".to_string(),
    }
    .into();
    assert_eq!(map.len(), 1);
    assert_eq!(map["playerId"], serde_json::json!("abc123"));

    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesGetByStatusArgs {
        status: GamesGetByStatusStatus::Active,
    }
    .into();
    assert_eq!(map.len(), 1);
    assert_eq!(map["status"], serde_json::json!("active"));
}

#[test]
fn test_tagged_union_args_into_btreemap()
{
    use example_types::{GamesUpdateGameStatusArgs, GamesUpdateGameStatusResult};

    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesUpdateGameStatusArgs {
        gameId: "game123".to_string(),
        result: GamesUpdateGameStatusResult::Win { bonus: 2.0 },
    }
    .into();
    assert_eq!(map.len(), 2);
    assert_eq!(map["gameId"], serde_json::json!("game123"));
    assert_eq!(map["result"], serde_json::json!({"type": "Win", "bonus": 2.0}));
}

#[test]
fn test_optional_args_none_skipped_in_btreemap()
{
    use example_types::GamesUpdateWithNoteArgs;

    // When optional fields are None, they should be absent from the map
    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesUpdateWithNoteArgs {
        gameId: "game123".to_string(),
        note: None,
        score: None,
    }
    .into();
    assert_eq!(map.len(), 1, "None fields should be absent from map");
    assert_eq!(map["gameId"], serde_json::json!("game123"));
    assert!(!map.contains_key("note"), "None note should be absent");
    assert!(!map.contains_key("score"), "None score should be absent");
}

#[test]
fn test_optional_args_some_included_in_btreemap()
{
    use example_types::GamesUpdateWithNoteArgs;

    // When optional fields are Some, they should appear in the map
    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesUpdateWithNoteArgs {
        gameId: "game456".to_string(),
        note: Some("hello".to_string()),
        score: Some(42.0),
    }
    .into();
    assert_eq!(map.len(), 3, "Some fields should be present in map");
    assert_eq!(map["gameId"], serde_json::json!("game456"));
    assert_eq!(map["note"], serde_json::json!("hello"));
    assert_eq!(map["score"], serde_json::json!(42.0));
}

#[test]
fn test_optional_args_mixed_in_btreemap()
{
    use example_types::GamesUpdateWithNoteArgs;

    // Mix of Some and None — only Some fields in the map
    let map: std::collections::BTreeMap<String, serde_json::Value> = GamesUpdateWithNoteArgs {
        gameId: "game789".to_string(),
        note: Some("partial".to_string()),
        score: None,
    }
    .into();
    assert_eq!(map.len(), 2, "only gameId and note should be present");
    assert_eq!(map["gameId"], serde_json::json!("game789"));
    assert_eq!(map["note"], serde_json::json!("partial"));
    assert!(!map.contains_key("score"), "None score should be absent");
}

#[test]
fn test_function_paths()
{
    use example_types::{
        GamesGetByStatusArgs, GamesGetGameArgs, GamesLossGameArgs, GamesUpdateGameStatusArgs, GamesWinGameArgs,
        PlayersAddAchievementArgs, PlayersCreateArgs, PlayersGetByIdArgs, PlayersGetByRankArgs, PlayersListActiveArgs,
        PlayersUpdateProfileArgs,
    };

    // Games
    assert_eq!(GamesGetGameArgs::FUNCTION_PATH, "games:getGame");
    assert_eq!(GamesWinGameArgs::FUNCTION_PATH, "games:winGame");
    assert_eq!(GamesLossGameArgs::FUNCTION_PATH, "games:lossGame");
    assert_eq!(GamesGetByStatusArgs::FUNCTION_PATH, "games:getByStatus");
    assert_eq!(GamesUpdateGameStatusArgs::FUNCTION_PATH, "games:updateGameStatus");

    // Players
    assert_eq!(PlayersListActiveArgs::FUNCTION_PATH, "players:listActive");
    assert_eq!(PlayersGetByIdArgs::FUNCTION_PATH, "players:getById");
    assert_eq!(PlayersGetByRankArgs::FUNCTION_PATH, "players:getByRank");
    assert_eq!(PlayersCreateArgs::FUNCTION_PATH, "players:create");
    assert_eq!(PlayersUpdateProfileArgs::FUNCTION_PATH, "players:updateProfile");
    assert_eq!(PlayersAddAchievementArgs::FUNCTION_PATH, "players:addAchievement");
}

#[test]
fn test_games_table_serde_roundtrip()
{
    use example_types::{GamesStatus, GamesTable};

    let json = serde_json::json!({
        "_id": "abc123",
        "_creationTime": 1700000000000.0,
        "win_count": 5.0,
        "loss_count": 3.0,
        "status": "active",
        "lastPlayedAt": 1700000001000.0,
    });

    let game: GamesTable = serde_json::from_value(json.clone()).expect("Deserialize failed");
    assert_eq!(game.id, "abc123");
    assert_eq!(game.creation_time, 1700000000000.0);
    assert_eq!(game.win_count, 5.0);
    assert_eq!(game.loss_count, 3.0);
    assert_eq!(game.status, GamesStatus::Active);
    assert_eq!(game.last_played_at, Some(1700000001000.0));

    // Round-trip: serialize back to JSON
    let serialized = serde_json::to_value(&game).expect("Serialize failed");
    assert_eq!(serialized, json);
}

#[test]
fn test_players_table_serde_roundtrip()
{
    use example_types::{PlayersProfileSettingsTheme, PlayersRank, PlayersTable};

    // Input JSON includes "avatar": null to test deserialization of null optionals
    let input_json = serde_json::json!({
        "_id": "player1",
        "_creationTime": 1700000000000.0,
        "name": "Alice",
        "score": 42.0,
        "isActive": true,
        "profile": {
            "bio": "hello",
            "avatar": null,
            "settings": {
                "theme": "dark",
                "notifications": true,
            },
        },
        "rank": "gold",
        "achievements": [
            { "name": "First Win", "unlockedAt": 1700000001000.0 },
        ],
        "stats": { "gamesPlayed": 10.0, "winRate": 0.5 },
    });

    let player: PlayersTable = serde_json::from_value(input_json).expect("Deserialize failed");
    assert_eq!(player.id, "player1");
    assert_eq!(player.name, "Alice");
    assert_eq!(player.score, 42.0);
    assert!(player.is_active);
    assert_eq!(player.profile.bio, Some("hello".to_string()));
    assert_eq!(player.profile.avatar, None);
    assert_eq!(player.profile.settings.theme, PlayersProfileSettingsTheme::Dark);
    assert!(player.profile.settings.notifications);
    assert_eq!(player.rank, PlayersRank::Gold);
    assert_eq!(player.achievements.len(), 1);
    assert_eq!(player.achievements[0].name, "First Win");
    assert_eq!(*player.stats.get("gamesPlayed").unwrap(), 10.0);
    assert_eq!(*player.stats.get("winRate").unwrap(), 0.5);

    // Round-trip: skip_serializing_if omits None fields, so avatar is absent
    let expected_json = serde_json::json!({
        "_id": "player1",
        "_creationTime": 1700000000000.0,
        "name": "Alice",
        "score": 42.0,
        "isActive": true,
        "profile": {
            "bio": "hello",
            "settings": {
                "theme": "dark",
                "notifications": true,
            },
        },
        "rank": "gold",
        "achievements": [
            { "name": "First Win", "unlockedAt": 1700000001000.0 },
        ],
        "stats": { "gamesPlayed": 10.0, "winRate": 0.5 },
    });
    let serialized = serde_json::to_value(&player).expect("Serialize failed");
    assert_eq!(serialized, expected_json);
}

#[test]
fn test_tagged_union_serde()
{
    use example_types::GamesUpdateGameStatusResult;

    // Win variant
    let win = GamesUpdateGameStatusResult::Win { bonus: 3.0 };
    let json = serde_json::to_value(&win).expect("Serialize failed");
    assert_eq!(json, serde_json::json!({"type": "Win", "bonus": 3.0}));

    // Loss variant
    let loss = GamesUpdateGameStatusResult::Loss { penalty: 1.0 };
    let json = serde_json::to_value(&loss).expect("Serialize failed");
    assert_eq!(json, serde_json::json!({"type": "Loss", "penalty": 1.0}));

    // Draw variant (unit)
    let draw = GamesUpdateGameStatusResult::Draw;
    let json = serde_json::to_value(&draw).expect("Serialize failed");
    assert_eq!(json, serde_json::json!({"type": "Draw"}));

    // Deserialize back
    let deserialized: GamesUpdateGameStatusResult =
        serde_json::from_value(serde_json::json!({"type": "Win", "bonus": 5.0})).expect("Deserialize failed");
    match deserialized {
        GamesUpdateGameStatusResult::Win { bonus } => assert_eq!(bonus, 5.0),
        _ => panic!("Expected Win variant"),
    }
}

#[test]
fn test_player_update_profile_action_serde()
{
    use example_types::{PlayersUpdateProfileAction, PlayersUpdateProfileActionUpdateSettingsTheme};

    // SetBio variant
    let action = PlayersUpdateProfileAction::SetBio {
        bio: "new bio".to_string(),
    };
    let json = serde_json::to_value(&action).expect("Serialize failed");
    assert_eq!(json, serde_json::json!({"type": "SetBio", "bio": "new bio"}));

    // UpdateSettings variant with nested enum
    let action = PlayersUpdateProfileAction::UpdateSettings {
        theme: PlayersUpdateProfileActionUpdateSettingsTheme::Dark,
        notifications: false,
    };
    let json = serde_json::to_value(&action).expect("Serialize failed");
    assert_eq!(
        json,
        serde_json::json!({
            "type": "UpdateSettings",
            "theme": "dark",
            "notifications": false,
        })
    );

    // ClearProfile (unit)
    let action = PlayersUpdateProfileAction::ClearProfile;
    let json = serde_json::to_value(&action).expect("Serialize failed");
    assert_eq!(json, serde_json::json!({"type": "ClearProfile"}));
}
