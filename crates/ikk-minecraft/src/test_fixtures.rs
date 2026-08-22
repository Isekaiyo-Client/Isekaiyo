//! Trimmed-but-realistic Mojang metadata fixtures. Tests never touch the
//! network: these mirror the real document shapes (version_manifest_v2 and a
//! modern `1.x` version JSON) with a handful of entries instead of hundreds.

/// Mirrors https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
pub const MANIFEST_JSON: &str = r#"{
  "latest": { "release": "1.21.4", "snapshot": "25w05a" },
  "versions": [
    {
      "id": "25w05a",
      "type": "snapshot",
      "url": "https://piston-meta.mojang.com/v1/packages/aa/25w05a.json",
      "time": "2025-01-29T10:00:00+00:00",
      "releaseTime": "2025-01-29T09:45:00+00:00"
    },
    {
      "id": "1.21.4",
      "type": "release",
      "url": "https://piston-meta.mojang.com/v1/packages/bb/1.21.4.json",
      "time": "2024-12-03T10:00:00+00:00",
      "releaseTime": "2024-12-03T09:40:00+00:00"
    },
    {
      "id": "1.20.4",
      "type": "release",
      "url": "https://piston-meta.mojang.com/v1/packages/cc/1.20.4.json",
      "time": "2023-12-07T12:00:00+00:00",
      "releaseTime": "2023-12-07T11:11:00+00:00"
    },
    {
      "id": "b1.7.3",
      "type": "old_beta",
      "url": "https://piston-meta.mojang.com/v1/packages/dd/b1.7.3.json",
      "time": "2020-06-01T10:00:00+00:00",
      "releaseTime": "2011-07-07T00:00:00+00:00"
    }
  ]
}"#;

/// Mirrors a modern (post-1.13 `arguments` format) version JSON, trimmed to
/// the fields Isekaiyo consumes: two plain libraries, one Linux-only native,
/// one Windows-only native, an asset index, a log4j config, Java 17.
pub const VERSION_METADATA_JSON: &str = r#"{
  "id": "1.20.4",
  "type": "release",
  "releaseTime": "2023-12-07T11:11:00+00:00",
  "mainClass": "net.minecraft.client.main.Main",
  "assets": "17",
  "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
  "arguments": {
    "game": [
      "--username", "${auth_player_name}",
      "--version", "${version_name}",
      "--gameDir", "${game_directory}",
      "--assetsDir", "${assets_root}",
      "--assetIndex", "${assets_index_name}",
      "--accessToken", "${auth_access_token}",
      { "rules": [{ "action": "allow", "features": { "is_demo_user": true } }], "value": "--demo" },
      { "rules": [{ "action": "allow", "os": { "name": "windows" } }], "value": ["--featuresRoot", "${game_directory}\\features"] }
    ],
    "jvm": [
      { "rules": [{ "action": "allow", "os": { "name": "osx" } }],
        "value": "-XstartOnFirstThread" },
      { "rules": [{ "action": "allow", "os": { "name": "windows" } }],
        "value": "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe" },
      "-Djava.library.path=${natives_directory}",
      "-Djna.tmpdir=${natives_directory}",
      "-cp", "${classpath}"
    ]
  },
  "assetIndex": {
    "id": "17",
    "url": "https://piston-meta.mojang.com/v1/assets/17/index.json",
    "sha1": "e8ba0d9cff10d10ffeb1c5bd8e2d0b8f9f7f2c4a",
    "size": 400000,
    "totalSize": 400000000
  },
  "downloads": {
    "client": {
      "url": "https://piston-data.mojang.com/v1/objects/aa/client.jar",
      "sha1": "3d50c9be6a2f0f1d0e0c9be6a2f0f1d0e0c9be6a",
      "size": 25000000
    }
  },
  "libraries": [
    {
      "name": "com.mojang:brigadier:1.1.8",
      "downloads": {
        "artifact": {
          "path": "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar",
          "url": "https://piston-data.mojang.com/libraries/com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar",
          "sha1": "e5b8ca8c3f0f1d0e0c9be6a2f0f1d0e0c9be6a2",
          "size": 80000
        }
      }
    },
    {
      "name": "org.lwjgl:lwjgl:3.3.1",
      "downloads": {
        "artifact": {
          "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar",
          "url": "https://piston-data.mojang.com/libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar",
          "sha1": "1111111111111111111111111111111111111111",
          "size": 700000
        }
      }
    },
    {
      "name": "org.lwjgl:lwjgl:3.3.1:natives-linux",
      "downloads": {
        "artifact": {
          "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar",
          "url": "https://piston-data.mojang.com/libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar",
          "sha1": "2222222222222222222222222222222222222222",
          "size": 90000
        }
      }
    },
    {
      "name": "org.lwjgl:lwjgl:3.3.1:natives-windows",
      "downloads": {
        "artifact": {
          "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar",
          "url": "https://piston-data.mojang.com/libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar",
          "sha1": "3333333333333333333333333333333333333333",
          "size": 90000
        }
      }
    },
    {
      "name": "com.example:legacy-only:1.0",
      "rules": [{ "action": "disallow", "os": { "name": "linux" } }],
      "downloads": {
        "artifact": {
          "path": "com/example/legacy-only/1.0/legacy-only-1.0.jar",
          "url": "https://example.invalid/legacy-only.jar",
          "sha1": "4444444444444444444444444444444444444444",
          "size": 10
        }
      }
    }
  ],
  "logging": {
    "client": {
      "argument": "-Dlog4j.configurationFile=${path}",
      "file": {
        "id": "client-1.12.xml",
        "url": "https://piston-data.mojang.com/assets/log-configs/client-1.12.xml",
        "sha1": "5555555555555555555555555555555555555555",
        "size": 900
      }
    }
  }
}"#;

/// Mirrors an asset index (`objects` map of name → {hash, size}).
pub const ASSET_INDEX_JSON: &str = r#"{
  "objects": {
    "minecraft/sounds/ambient/cave/cave1.ogg": {
      "hash": "4c8a5e7f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d",
      "size": 123456
    },
    "minecraft/lang/en_us.json": {
      "hash": "aa6b8f7c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a",
      "size": 8000
    }
  }
}"#;
