import assert from "node:assert/strict";
import test from "node:test";
import { computeTonearmPose } from "./deckPose.ts";
import { derivePlaybackMode, nextPlaybackMode, playbackModeLabel } from "./playbackMode.ts";
import { artistLink, trackLink } from "./spotifyLinkUrls.ts";

test("playback mode derives visual state from Spotify shuffle and repeat flags", () => {
  assert.equal(derivePlaybackMode(null, null), null);
  assert.equal(derivePlaybackMode(true, null), null);
  assert.equal(derivePlaybackMode(false, "off"), "off");
  assert.equal(derivePlaybackMode(true, "off"), "shuffle");
  assert.equal(derivePlaybackMode(true, "context"), "shuffle");
  assert.equal(derivePlaybackMode(false, "context"), "repeatContext");
  assert.equal(derivePlaybackMode(false, "track"), "repeatTrack");
  assert.equal(derivePlaybackMode(true, "track"), "repeatTrack");
});

test("playback mode cycles Shuffle -> Repeat Context -> Repeat Track -> Shuffle", () => {
  assert.deepEqual(nextPlaybackMode("off"), { shuffle: true, repeat: "off" });
  assert.deepEqual(nextPlaybackMode("shuffle"), { shuffle: false, repeat: "context" });
  assert.deepEqual(nextPlaybackMode("repeatContext"), { shuffle: false, repeat: "track" });
  assert.deepEqual(nextPlaybackMode("repeatTrack"), { shuffle: true, repeat: "off" });
  assert.deepEqual(nextPlaybackMode(null), { shuffle: true, repeat: "off" });
});

test("every playback mode has a non-empty accessible label", () => {
  for (const mode of ["off", "shuffle", "repeatContext", "repeatTrack", null]) {
    assert.ok(playbackModeLabel(mode).length > 0);
  }
});

test("tonearm rests fully on arm rest when paused and nudges slightly on hover", () => {
  const pausedRest = computeTonearmPose({
    spotifyRunning: true,
    playback: "paused",
    progress: 0.5,
    hovered: false,
  });
  assert.deepEqual(pausedRest, { angleDeg: 0, lifted: false, transition: "rest" });

  const pausedHover = computeTonearmPose({
    spotifyRunning: true,
    playback: "paused",
    progress: 0.5,
    hovered: true,
  });
  assert.equal(pausedHover.lifted, true);
  assert.ok(pausedHover.angleDeg > 0 && pausedHover.angleDeg < 21);
  assert.equal(pausedHover.transition, "hover");
});

test("tonearm tracks progress while playing and nudges toward rest on hover", () => {
  const playing = computeTonearmPose({
    spotifyRunning: true,
    playback: "playing",
    progress: 0.5,
    hovered: false,
  });
  assert.equal(playing.angleDeg, 27.5);
  assert.equal(playing.lifted, false);
  assert.equal(playing.transition, "play");

  const playingHover = computeTonearmPose({
    spotifyRunning: true,
    playback: "playing",
    progress: 0.5,
    hovered: true,
  });
  assert.equal(playingHover.lifted, true);
  assert.ok(playingHover.angleDeg < playing.angleDeg);
  assert.equal(playingHover.transition, "hover");
});

test("tonearm stays resting when Spotify is not running", () => {
  const closed = computeTonearmPose({
    spotifyRunning: false,
    playback: "stopped",
    progress: 0,
    hovered: true,
  });
  assert.deepEqual(closed, { angleDeg: 0, lifted: false, transition: "rest" });
});

test("artistLink uses direct Spotify artist id when available and falls back to search", () => {
  const withId = artistLink({
    spotifyId: "t1",
    title: "Song",
    artists: ["Low Hollow", "Guest"],
    artistId: "a1",
    album: "Album",
    albumId: "al1",
    durationMs: 1000,
    art: null,
  });
  assert.deepEqual(withId, {
    app: "spotify:artist:a1",
    web: "https://open.spotify.com/artist/a1",
  });

  const withoutId = artistLink({
    spotifyId: null,
    title: "Song",
    artists: ["Low Hollow"],
    artistId: null,
    album: null,
    albumId: null,
    durationMs: null,
    art: null,
  });
  assert.deepEqual(withoutId, {
    app: "spotify:search:Low%20Hollow",
    web: "https://open.spotify.com/search/Low%20Hollow/artists",
  });

  const noArtists = artistLink({
    spotifyId: null,
    title: "Song",
    artists: [],
    artistId: null,
    album: null,
    albumId: null,
    durationMs: null,
    art: null,
  });
  assert.equal(noArtists, null);
});

test("trackLink uses direct Spotify track id when available and falls back to search", () => {
  const withId = trackLink({
    spotifyId: "t1",
    title: "Song",
    artists: ["Low Hollow"],
    artistId: "a1",
    album: null,
    albumId: null,
    durationMs: null,
    art: null,
  });
  assert.deepEqual(withId, {
    app: "spotify:track:t1",
    web: "https://open.spotify.com/track/t1",
  });
});
