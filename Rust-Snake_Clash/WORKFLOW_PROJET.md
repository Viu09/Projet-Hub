# Workflow du projet Snake Clash MVP

## 1) Objectif global
Le projet sépare l'application en **3 blocs** :
- **Client jeu** (interface + rendu + inputs) : `macroquad`
- **Master Server** (annuaire des rooms) : API HTTP `axum` sur `:9100`
- **Game Server** (temps réel) : WebSocket `tokio-tungstenite` sur `:9001`

## 2) Parcours utilisateur (vision client)
1. Le joueur lance le client.
2. Le client affiche la liste des rooms via le **Master Server** (`GET /rooms`).
3. Le joueur peut :
- rejoindre une room existante,
- créer une room,
- supprimer une room (mode local/dev).
4. Une fois la room choisie, le client se connecte au **Game Server** en WebSocket.
5. Le serveur valide la connexion (`JoinOk`) puis lance la synchronisation en continu (snapshots).
6. Pendant la partie, le client envoie les inputs (direction, boost), le serveur calcule l'état autoritaire, puis renvoie l'état du match.

## 3) Rôle de chaque composant

### Client
- UI lobby/menu : `src/client/lobby_ui.rs`
- Appels HTTP vers master : `src/client/master_api.rs`
- Connexion WS : `src/client/net.rs`
- Buffer d'état réseau (snapshots/deltas) : `src/client/state.rs`
- Orchestration runtime client : `src/client/runtime.rs`
- Boucle de jeu/rendu : `src/game/loop.rs`

### Master Server (HTTP)
- Démarrage : `master::serve("0.0.0.0:9100")`
- Routes : `src/master/routes.rs`
- État des rooms en mémoire : `src/master/state.rs`
- Utilité : fournir au client une liste de rooms + adresses de serveur de jeu.

### Game Server (WebSocket)
- Démarrage : `WsServer::serve("0.0.0.0:9001", dispatcher)`
- Gestion des sessions + routage : `src/net/ws.rs`, `src/net/dispatcher.rs`
- Logique lobby/rooms côté serveur : `src/state/lobby.rs`
- Simulation autoritaire d'une room : `src/state/room.rs`

## 4) Flux réseau (important pour le client)

### A. Matchmaking (HTTP)
- `GET /rooms` : récupère les rooms disponibles.
- `POST /rooms` : crée une room.
- `DELETE /rooms/:room_id` : supprime une room.

### B. Partie temps réel (WebSocket)
Messages client -> serveur (`ClientMessage`) :
- `join_req` : entrer dans une room
- `input` : direction/boost + ack du dernier snapshot
- `ping`, `leave`

Messages serveur -> client (`ServerMessage`) :
- `join_ok` : confirmation + `player_id`
- `snapshot` : état complet
- `snapshot_delta` : état partiel (optimisé)
- `pong`, `player_left`

Le protocole est sérialisé en **binaire (bincode)** avec fallback **JSON** (`src/net/codec.rs`).

## 5) Boucle de synchronisation
- Le Game Server tick toutes les **50 ms** (20 Hz).
- À chaque tick :
1. le serveur applique les inputs reçus,
2. calcule collisions, score, tokens, chrono,
3. envoie un `snapshot` ou `snapshot_delta` aux clients.
- Le client applique les snapshots, met à jour l'affichage et continue d'envoyer les inputs.

## 6) Autorité serveur
Le serveur est **source de vérité** :
- positions et états des joueurs,
- collisions,
- bonus/tokens,
- score et timer.

Le client est surtout responsable de :
- l'interface,
- le ressenti des contrôles,
- l'affichage en temps réel.

## 7) Lancement en local (démo)

### Option simple (server + master ensemble)
- `cargo run -- server`

Cela démarre :
- Master API sur `http://127.0.0.1:9100`
- Game WS sur `ws://127.0.0.1:9001`

### Client
- dans un autre terminal : `cargo run -- client`

## 8) Message clé pour ton client
Le projet implémente une architecture multijoueur classique et saine :
- **Master** pour découvrir/organiser les parties,
- **Game Server autoritaire** pour la simulation,
- **Client léger** pour l'UX et le rendu.

Cette séparation rend le système plus clair, scalable et maintenable.
