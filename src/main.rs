use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use dotenv::dotenv;
use std::error::Error;
use base64::{Engine as _, engine::general_purpose};
use std::io::{Write, BufRead, BufReader};
use std::net::TcpListener;
use std::process::Command;
use url::Url;
use rand::Rng;

#[derive(Debug, Deserialize)]
struct SpotifyToken {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct AuthToken {
    access_token: String,
    refresh_token: String,
    expires_in: u32,
}

#[derive(Debug, Serialize)]
struct CreatePlaylistRequest {
    name: String,
    description: String,
    public: bool,
}

#[derive(Debug, Deserialize)]
struct SpotifyPlaylist {
    id: String,
    external_urls: ExternalUrls,
}

#[derive(Debug, Deserialize)]
struct ExternalUrls {
    spotify: String,
}

#[derive(Debug, Deserialize)]
struct SpotifySearchResponse {
    albums: SpotifyAlbums,
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbums {
    items: Vec<SpotifyAlbum>,
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbum {
    uri: String,
}

// Simplified Last.fm structures
#[derive(Debug, Deserialize)]
struct TopAlbums {
    topalbums: TopAlbumsContent,
}

#[derive(Debug, Deserialize)]
struct TopAlbumsContent {
    album: Vec<TopAlbum>,
}

#[derive(Debug, Deserialize)]
struct TopAlbum {
    name: String,
    artist: TopArtist,
}

#[derive(Debug, Deserialize)]
struct TopArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SimilarArtists {
    similarartists: Similar,
}

#[derive(Debug, Deserialize)]
struct Similar {
    artist: Vec<SimilarArtist>,
}

#[derive(Debug, Deserialize)]
struct SimilarArtist {
    name: String,
}

async fn get_spotify_auth_token(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<String, Box<dyn Error>> {
    // Generate a random state string
    let state: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    // Set up local server to receive the callback
    let listener = TcpListener::bind("127.0.0.1:8888")?;
    println!("Started local server on port 8888");

    // Construct the authorization URL
    let scopes = ["playlist-modify-private", "playlist-modify-public"];
    let auth_url = format!(
        "https://accounts.spotify.com/authorize?client_id={}\
         &response_type=code\
         &redirect_uri={}\
         &scope={}\
         &state={}",
        client_id,
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&scopes.join(" ")),
        state
    );

    // Open the default web browser
    #[cfg(target_os = "windows")]
    Command::new("cmd").args(["/C", "start", &auth_url]).spawn()?;
    #[cfg(target_os = "macos")]
    Command::new("open").arg(&auth_url).spawn()?;
    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(&auth_url).spawn()?;

    println!("Please authorize the application in your browser...");

    // Wait for the callback
    let (mut stream, _) = listener.accept()?;
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Extract the authorization code from the callback URL
    let redirect_url = request_line.split_whitespace().nth(1)
        .ok_or("Invalid request")?;
    let url = Url::parse(&format!("http://localhost{}", redirect_url))?;
    let code = url.query_pairs()
        .find(|(key, _)| key == "code")
        .ok_or("No code found")?
        .1
        .into_owned();

    // Send success response to browser
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                   <html><body><h1>Authorization successful!</h1>\
                   <p>You can close this window now.</p></body></html>";
    stream.write_all(response.as_bytes())?;

    // Exchange the code for an access token
    let client = Client::new();
    let auth = general_purpose::STANDARD.encode(format!("{}:{}", client_id, client_secret));
    
    let token_response = client
        .post("https://accounts.spotify.com/api/token")
        .header("Authorization", format!("Basic {}", auth))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await?;
        return Err(format!("Failed to get token: {}", error_text).into());
    }

    let auth_token: AuthToken = token_response.json().await?;
    Ok(auth_token.access_token)
}

async fn get_recommendations(
    client: &Client,
    username: &str,
    api_key: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut recommendations = Vec::new();
    let mut seen_artists = HashSet::new();
    
    // Get top albums from last 6 months only
    let url = format!(
        "http://ws.audioscrobbler.com/2.0/?method=user.gettopalbums&user={}&api_key={}&format=json&period=6month&limit=10",
        username, api_key
    );
    
    println!("📊 Fetching your top albums...");
    let top_albums: TopAlbums = client.get(&url).send().await?.json().await?;
    
    // Process each top artist
    for album in &top_albums.topalbums.album {
        if seen_artists.contains(&album.artist.name) {
            continue;
        }
        seen_artists.insert(album.artist.name.clone());
        
        println!("🔍 Finding similar artists to: {}", album.artist.name);
        
        // Get similar artists
        let similar_url = format!(
            "http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json&limit=5",
            urlencoding::encode(&album.artist.name), api_key
        );
        
        if let Ok(similar) = client.get(&similar_url).send().await?.json::<SimilarArtists>().await {
            // Get top album from each similar artist
            for similar_artist in similar.similarartists.artist.iter().take(2) {
                let artist_albums_url = format!(
                    "http://ws.audioscrobbler.com/2.0/?method=artist.gettopalbums&artist={}&api_key={}&format=json&limit=1",
                    urlencoding::encode(&similar_artist.name), api_key
                );
                
                if let Ok(artist_albums) = client.get(&artist_albums_url).send().await?.json::<TopAlbums>().await {
                    if let Some(top_album) = artist_albums.topalbums.album.first() {
                        recommendations.push((similar_artist.name.clone(), top_album.name.clone()));
                        println!("✓ Added recommendation: {} - {}", similar_artist.name, top_album.name);
                        
                        if recommendations.len() >= 10 {
                            return Ok(recommendations);
                        }
                    }
                }
            }
        }
    }
    
    Ok(recommendations)
}

async fn create_spotify_playlist(
    token: &str,
    user_id: &str,
    recommendations: &[(String, String)],
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    
    println!("Creating playlist...");
    
    // Create playlist
    let playlist_response = client
        .post(&format!(
            "https://api.spotify.com/v1/users/{}/playlists",
            user_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&CreatePlaylistRequest {
            name: format!("Last.fm Discoveries - {}", chrono::Local::now().format("%Y-%m-%d")),
            description: "Fresh music recommendations based on your Last.fm history".to_string(),
            public: false,
        })
        .send()
        .await?;

    // Add error handling for non-successful responses
    let status = playlist_response.status();
    if !status.is_success() {
        let error_text = playlist_response.text().await?;
        return Err(format!("Failed to create playlist. Status: {}, Error: {}", 
            status, error_text).into());
    }

    let playlist: SpotifyPlaylist = playlist_response.json().await?;
    
    let playlist_url = playlist.external_urls.spotify.clone();
    println!("Created playlist: {}", playlist_url);
    
    // Find and add tracks (not albums) to the playlist
    let mut track_uris = Vec::new();
    
    for (artist, album) in recommendations {
        let query = format!("album:{} artist:{}", album, artist);
        let search_url = format!(
            "https://api.spotify.com/v1/search?q={}&type=album,track&limit=1",
            urlencoding::encode(&query)
        );
        
        let search_response = client
            .get(&search_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = search_response.status();
        if !status.is_success() {
            println!("Warning: Search failed for {} - {}", artist, album);
            continue;
        }

        #[derive(Debug, Deserialize)]
        struct SearchResponse {
            tracks: Option<TracksResponse>,
            albums: SpotifyAlbums,
        }

        #[derive(Debug, Deserialize)]
        struct TracksResponse {
            items: Vec<Track>,
        }

        #[derive(Debug, Deserialize)]
        struct Track {
            uri: String,
        }

        let search_result: SearchResponse = search_response.json().await?;
        
        // Try to get the first track from the album
        if let Some(tracks) = search_result.tracks {
            if let Some(track) = tracks.items.first() {
                track_uris.push(track.uri.clone());
                println!("Found track on Spotify: {} - {}", artist, album);
            }
        } else if let Some(album_result) = search_result.albums.items.first() {
            // If no track found, get tracks from the album
            let album_tracks_url = format!(
                "https://api.spotify.com/v1/albums/{}/tracks?limit=1",
                album_result.uri.split(":").last().unwrap_or("")
            );
            
            let tracks_response = client
                .get(&album_tracks_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            if let Ok(response) = tracks_response {
                if let Ok(tracks) = response.json::<TracksResponse>().await {
                    if let Some(track) = tracks.items.first() {
                        track_uris.push(track.uri.clone());
                        println!("Found album track on Spotify: {} - {}", artist, album);
                    }
                }
            }
        }
    }
    
    // Add tracks to playlist in batches
    if !track_uris.is_empty() {
        println!("Adding tracks to playlist...");
        
        // Spotify allows maximum 100 tracks per request
        for chunk in track_uris.chunks(100) {
            let add_response = client
                .post(&format!(
                    "https://api.spotify.com/v1/playlists/{}/tracks",
                    playlist.id
                ))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .json(&chunk)
                .send()
                .await?;

            let status = add_response.status();
            if !status.is_success() {
                println!("Warning: Failed to add some tracks to playlist");
            }
        }
    } else {
        println!("Warning: No tracks found to add to playlist");
    }
    
    Ok(playlist_url)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    
    let lastfm_key = std::env::var("LASTFM_API_KEY")?;
    let lastfm_user = std::env::var("LASTFM_USERNAME")?;
    let spotify_client_id = std::env::var("SPOTIFY_CLIENT_ID")?;
    let spotify_client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")?;
    let spotify_user_id = std::env::var("SPOTIFY_USER_ID")?;
    
    // Use a fixed redirect URI - add this to your Spotify app settings
    let redirect_uri = "http://localhost:8888/callback";
    
    let client = Client::new();
    
    // Get recommendations
    let recommendations = get_recommendations(&client, &lastfm_user, &lastfm_key).await?;
    
    if recommendations.is_empty() {
        println!("❌ Couldn't find any recommendations.");
        return Ok(());
    }
    
    println!("\n✨ Found these recommendations:");
    for (i, (artist, album)) in recommendations.iter().enumerate() {
        println!("{}. {} - {}", i + 1, artist, album);
    }
    
    // Get authorized token using OAuth flow
    println!("\n🔐 Starting Spotify authorization...");
    let spotify_token = get_spotify_auth_token(
        &spotify_client_id,
        &spotify_client_secret,
        redirect_uri,
    ).await?;

    // Create the playlist
    match create_spotify_playlist(&spotify_token, &spotify_user_id, &recommendations).await {
        Ok(playlist_url) => {
            println!("\n✅ Successfully created Spotify playlist!");
            println!("🎵 Open your playlist here: {}", playlist_url);
            
            #[cfg(target_os = "windows")]
            {
                println!("\nOpening playlist in your browser...");
                Command::new("cmd")
                    .args(["/C", "start", &playlist_url])
                    .spawn()?;
            }
            
            #[cfg(target_os = "macos")]
            {
                println!("\nOpening playlist in your browser...");
                Command::new("open")
                    .arg(&playlist_url)
                    .spawn()?;
            }
            
            #[cfg(target_os = "linux")]
            {
                println!("\nOpening playlist in your browser...");
                Command::new("xdg-open")
                    .arg(&playlist_url)
                    .spawn()?;
            }
        }
        Err(e) => println!("\n❌ Failed to create Spotify playlist: {}", e),
    }
    
    Ok(())
}