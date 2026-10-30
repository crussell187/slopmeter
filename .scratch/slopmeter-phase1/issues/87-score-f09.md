# Score an inline I/O dependency as F09

Type: task
Status: resolved
Blocked by: 

## Question

Build F09 per [ADR 0036](../../../docs/adr/0036-three-warns-where-they-apply.md) in all three adapters. Each adapter records calls whose written name is on that language's default list (Python: `open`, `sqlite3.connect`, `socket.socket`, `subprocess.run`, `subprocess.Popen`, `urllib.request.urlopen`, `requests.Session`, `httpx.Client`, `httpx.AsyncClient`; Rust: `File::open`, `File::create`, `TcpStream::connect`, `TcpListener::bind`, `Command::new`, `reqwest::Client::new`, `reqwest::blocking::Client::new`; C++: `std::ifstream`, `std::ofstream`, `std::fstream`, `fopen`, `socket`, `popen`, `system`), `rules::evaluate` emits one weight-1 warn at the first such call, `slopmeter.toml` takes `[F09] allow` per language with a departure note per name, the explain page is written, and each sloppy corpus gains one inline open or connect.

## Answer

Resolved 2026-10-30. Each adapter records a call whose written name is on that language's list, and a nested function keeps its own calls. `rules::evaluate` emits one weight-1 warn at the first call that is not named in `[profile.<language>.F09] allow`. The measured value is the count of those calls and the threshold is 0. Each allowed name is its own departure note. A waiver line for F09 fails the run. `explain F09` says the threshold is any. Each sloppy corpus gained one `open`, `File::open`, or `fopen`. `slopmeter --strict` on this repo still exits 0.
