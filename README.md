# Last-Spot 🎵

Last-Spot is a Rust application that creates personalized Spotify playlists based on your Last.fm listening history. It analyzes your top artists, finds similar artists you might enjoy, and automatically creates a Spotify playlist with their top tracks.

## Features ✨
- Analyzes your Last.fm listening history from the past 6 months
- Discovers new artists similar to your favorites
- Automatically creates a private Spotify playlist
- Randomly selects recommendations for variety
- Opens the playlist in your browser when done

## Prerequisites 

Before running Last-Spot, you'll need:

1. A Last.fm API account:
   - Sign up at [Last.fm API](https://www.last.fm/api/account/create)
   - Get your API key

2. A Spotify Developer account:
   - Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Create a new application
   - Get your Client ID and Client Secret
   - Add `http://localhost:8888/callback` to your Redirect URIs in the app settings

## Installation 

1. Clone the repository:
```bash
git clone https://github.com/yourusername/last-spot.git
cd last-spot
```

2. Create a `.env` file in the project root with your credentials:
```env
LASTFM_API_KEY=your_lastfm_api_key
LASTFM_USERNAME=your_lastfm_username
SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
SPOTIFY_USER_ID=your_spotify_username
```

3. Build the project:
```bash
cargo build --release
```

## Usage 

Run the program:
```bash
cargo run
```

The application will:
1. Fetch your top albums from Last.fm
2. Find similar artists
3. Open your browser for Spotify authorization
4. Create a new private playlist
5. Add discovered tracks to the playlist
6. Open the playlist in your browser

## How ?

1. **Last.fm Analysis:**
   - Fetches your top albums from the last 6 months
   - Identifies unique artists from these albums

2. **Discovery:**
   - For each top artist, finds similar artists
   - Gets top albums from these similar artists
   - Randomly selects tracks to ensure variety

3. **Spotify Integration:**
   - Creates a new private playlist
   - Searches for each recommended track
   - Adds found tracks to the playlist

## Dependencies 

```toml
[dependencies]
tokio = { version = "1.28", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dotenv = "0.15"
base64 = "0.21"
urlencoding = "2.1"
chrono = "0.4"
rand = "0.8"
url = "2.4"
```

## Contributing 🤝

Please help test this out! 

