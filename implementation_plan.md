# Master Architecture & Implementation Plan: AOR Game Studio

Un environnement de développement complet (Game Engine Studio / IDE) bâti sur notre moteur 3D Rust / WGPU existant. Ce plan détaille l'ensemble des modules, structures de données, graphes nodaux, pipelines de compilation WGSL, formats de fichiers et interfaces de travail pour permettre une exécution autonome et rigoureuse sans ambiguïté.

---

## Architecture Générale & Flux de Données

```mermaid
graph TD
    subgraph Core Engine Layer
        WGPU[Renderer WGPU PBR / Raytracing]
        ECS[ECS Registry & Hierarchy]
        Audio[Spatial Audio 3D]
        Physics[Flight & Collision Engine]
        ScriptVM[Rhai VM & DSL Interpreter]
    end

    subgraph Studio Engine Subsystems
        VFS[1. Asset Pipeline & Virtual FileSystem (UUID / .meta)]
        NodeMat[2. Node Graph Texture & Shader Compiler (WGSL Live)]
        PrefabSys[3. Prefab & World Level Designer (Snapping / Gizmos)]
        EventLogic[4. Visual Logic Graph & State Machine]
        Packager[5. Project Packager & Standalone Game Exporter]
    end

    subgraph Cyber-Glass Studio UI (ui_gpu)
        ProjectHub[Project Hub & Launcher]
        AssetBrowser[Asset Browser & Thumbnail Cache]
        MaterialEditor[Node Graph Canvas & Live Preview Spheres]
        SceneHierarchy[Scene Tree & Inspector Panel]
        Viewport3D[Interactive 3D Viewport with Gizmos & Snapping]
        ConsoleLogs[Log Console & Rhai Interactive Terminal]
    end

    VFS --> AssetBrowser
    NodeMat --> MaterialEditor
    NodeMat --> WGPU
    PrefabSys --> SceneHierarchy
    PrefabSys --> Viewport3D
    EventLogic --> ScriptVM
    Packager --> Core Engine Layer
```

---

## 1. Asset Pipeline & Système de Fichiers Virtuel (VFS)

### 1.1 Invariants & Spécifications Techniques
* **Structure Projet** : Tout projet de jeu est un dossier autonome contenant :
  * `Project.aorproj` : Manifeste du projet (Nom, Version, Scène de démarrage, Paramètres de rendu).
  * `/Assets/` : Ressources sources (`.png`, `.jpg`, `.gltf`, `.obj`, `.wav`, `.ogg`, `.rhai`, `.aormat`, `.aorprefab`, `.aorscene`).
  * `/Assets/**/*.meta` : Fichier JSON généré automatiquement avec un **UUIDv4** unique et les options d'importation.
  * `/Library/Cache/` : Données binaires optimisées (textures compressées/mipmaps GPU, meshs vertex buffer packés, shaders précompilés).
* **Hot-Reloading & File Watcher** : Utilisation de `notify` pour détecter les modifications sur disque et recharger les assets à chaud sans redémarrer le studio.

### 1.2 Structure des Données Asset

```rust
use uuid::Uuid;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId(pub Uuid);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AssetType {
    Texture2D,
    Mesh,
    MaterialNodeGraph,
    Shader,
    AudioClip,
    RhaiScript,
    Prefab,
    Scene,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub asset_type: AssetType,
    pub source_path: PathBuf,
    pub import_settings: ImportSettings,
    pub hash: u64, // Hash Blake3 pour vérification de cache
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ImportSettings {
    Texture { generate_mipmaps: bool, srgb: bool, filter_mode: u8 },
    Mesh { calculate_tangents: bool, flip_uv: bool, optimize_indices: bool },
    Audio { streaming: bool, spatial_3d: bool, volume: f32 },
    Generic,
}
```

---

## 2. Éditeur de Textures et Matériaux par Nœuds (Shader Node Graph)

L'éditeur de shaders/textures permet de concevoir visuellement des surfaces PBR complexes, des bruits procéduraux, des masques et des animations de coordonnées UV avec **compilation dynamique en WGSL** et prévisualisation temps réel.

### 2.1 Types de Nœuds et Schéma de Connexion

```mermaid
graph LR
    UV[UV Coordinates] --> Scale[Scale / Offset]
    Scale --> Noise[Perlin / Voronoi Noise]
    ColorA[Color Constant (Cyan)] --> Lerp[Mix / Lerp]
    ColorB[Color Constant (Dark)] --> Lerp
    Noise --> Lerp
    Lerp --> Albedo[Material Output: Albedo]
    
    Fresnel[Fresnel Effect] --> Emissive[Material Output: Emissive]
    RoughnessSlider[Constant (0.2)] --> Roughness[Material Output: Roughness]
```

### 2.2 Arbre de Types de Nœuds

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NodeCategory {
    Input,      // UV, Position Monde, Normale Monde, View Dir, Time, VertexColor
    Math,       // Add, Sub, Mul, Div, Sin, Cos, Pow, Clamp, Mix/Lerp, Remap
    Procedural, // PerlinNoise, SimplexNoise, Voronoi/Cellular, Checkerboard, Gradient
    Color,      // RGB, RGBA, ColorRamp, HSV, Desaturate, Invert
    Texture,    // SampleTexture2D, NormalMapUnpack, TriplanarMapping
    Effect,     // Fresnel, ParallaxOcclusion, DistanceToEdge, WaveDisplacement
    Output,     // PBR Material Output (Albedo, Normal, Metallic, Roughness, Emissive, AO, Alpha)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodePin {
    pub id: Uuid,
    pub name: String,
    pub data_type: PinDataType, // Float, Vec2, Vec3, Vec4, Texture2D, Sampler
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeLink {
    pub id: Uuid,
    pub from_node: Uuid,
    pub from_pin: Uuid,
    pub to_node: Uuid,
    pub to_pin: Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialNodeGraph {
    pub id: AssetId,
    pub name: String,
    pub nodes: HashMap<Uuid, GraphNode>,
    pub links: Vec<NodeLink>,
    pub output_node_id: Uuid,
}
```

### 2.3 Compilateur de Graphe vers Code WGSL
Le compilateur effectue un **tri topologique (DAG traversal)** depuis le nœud de sortie `Material Output` vers les feuilles. Pour chaque nœud :
1. Génère les variables intermédiaires locales (ex: `let n_3_out = mix(n_1_out, n_2_out, n_0_out);`).
2. Gère les fonctions mathématiques WGSL intégrées et injecte les helpers de bruit procédural (Perlin/Voronoi) si utilisés.
3. Construit la fonction fragment `fs_pbr_material(in: VertexOutput) -> FragmentOutput`.
4. Compile à chaud le pipeline WGPU (`device.create_shader_module` + `device.create_render_pipeline`) et l'assigne en 0ms au preview sphere/cube.

---

## 3. Éditeur de Monde, Scènes & Système de Prefabs

### 3.1 Système de Prefabs
* Un **Prefab** (`.aorprefab`) est un graphe hiérarchique d'entités avec leurs composants sérialisés (Mesh, Material, Transform, Colliders, Scripts, Lights, Particles).
* **Prefab Overrides** : Instancier un Prefab dans une scène crée un lien vers l'asset maître tout en autorisant des surcharges locales (position, couleur néon spécifique) sans dupliquer le template.

### 3.2 Outils de Level Design (Interactive Viewport)
* **Translation / Rotation / Scale Gizmos** avec magnétisme ajustable (Snap Grille : 0.5m, 1m, 5m / Snap Angle : 15°, 45°, 90°).
* **Surface Raycast Snapping** : Déplacer un objet le colle automatiquement à la surface du sol ou du bâtiment rencontré par le raycast de la souris.
* **Scatter / Duplication Tool** : Pinceau pour peindre rapidement des lampadaires, débris urbains, enseignes néons ou anneaux le long d'une spline / trajectoire.
* **Spline / Road Curve Tool** : Générateur procédural d'arches de tunnels, rails aériens et pistes de course le long de points de contrôle de Bézier 3D.

---

## 4. Scripting Rhai, Machines à États & Visual Logic Triggers

### 4.1 Visual Triggers & Event Nodes
Pour prototyper un jeu sans écrire de code ou pour déclencher des cinématiques :
* **Trigger Volumes** : Box / Sphere Collider avec trigger flag `is_trigger: true`.
* **Événements disponibles** : `OnEnter`, `OnExit`, `OnStay`, `OnInteract (Key/Button)`, `OnTimer`, `OnScoreChange`.
* **Actions chaînables** : `PlaySound(id, pos)`, `SpawnPrefab(id, transform)`, `EmitParticles(id, count)`, `CameraShake(intensity, duration)`, `AddScore(points)`, `LoadScene(name)`.

### 4.2 Scripting Rhai avec Live Debugging
* Éditeur de script intégré dans le studio avec coloration syntaxique, numérotation des lignes et détection d'erreurs en direct.
* Console REPL permettant d'exécuter des commandes Rhai directement dans le jeu en cours de test (ex: `player.boost = 100.0; spawn_drone();`).

---

## 5. Gestionnaire de Projet & Packager / Exporteur de Jeux

### 5.1 Architecture du Studio UI (Cyber-Glass Layout)

L'interface du studio s'organise en docks redimensionnables :
```
+-----------------------------------------------------------------------------------+
| [File] [Edit] [Assets] [GameObject] [NodeGraph] [Build] | [Play >] [Pause] [Stop] |
+-----------------------+-----------------------------------+-----------------------+
|  SCENE HIERARCHY      |         3D VIEWPORT /             |  INSPECTOR            |
|  - World Root         |       NODE GRAPH CANVAS           |  - Transform          |
|    - CyberCity        |  (Switchable tabs:                |    Pos: [ 0, 10, 0 ]  |
|      - Skyscraper_01  |   [3D Scene] [Shader Editor]      |    Rot: [ 0,  0, 0 ]  |
|      - NeonTunnel_A   |   [Prefab View] [Rhai Editor])    |  - PBR Material Node  |
|    - PlayerSpinner    |                                   |  - Flight Controller  |
|    - Checkpoint_01    |                                   |  - Audio Emitter      |
+-----------------------+-----------------------------------+-----------------------+
|  ASSET BROWSER (VFS)                                      |  CONSOLE & TERMINAL   |
|  [📂 Assets] > Models > Buildings                         |  [Info] Engine Ready  |
|  [📄 Skyscraper.gltf] [🎨 NeonGlow.aormat] [🚗 Spinner]    |  > rhai: speed = 250  |
+-----------------------------------------------------------+-----------------------+
```

### 5.2 Packager Autonome (Export du Jeu Final)
* Bouton **"Build Standalone Game"** :
  1. Parcourt les assets référencés dans la scène de démarrage et ses dépendances.
  2. Compile les shaders de graphes en WGSL packé et binarise les meshs/textures dans une archive unique `.aorpak`.
  3. Compile ou copie le binaire `runner` optimisé qui charge directement le `.aorpak` sans les dépendances d'UI du studio.

---

## Plan d'Implémentation Séquentiel (Phases de Réalisation)

### Phase 1 : Asset Database, VFS & Modèle de Projet
- [x] Création du module `src/studio/vfs/` : structure de dossiers, fichiers `.meta`, générateur d'UUID et scanner de répertoire.
- [x] Système de sérialisation / désérialisation de projet (`ProjectManifest`, `SceneFile`).

### Phase 2 : Moteur de Graphes de Nœuds (Node Graph Core & WGSL Generator)
- [x] Création du module `src/studio/nodes/` : structures `GraphNode`, `NodePin`, `NodeLink`, `GraphCompiler`.
- [x] Bibliothèque de nœuds mathématiques, coordonnées UV, générateurs de bruit (Perlin, Voronoi) et sorties PBR.
- [x] Compilateur WGSL temps réel avec compilation WGPU et validation Naga sans crash.

### Phase 3 : Interface Visuelle du Studio (Docks & Canvas)
- [x] Création du module `src/studio/ui/` avec notre backend `ui_gpu` :
  - Canvas interactif pour l'éditeur de nœuds (zoom, pan, connexions de câbles bezier lumineux).
  - Explorateur d'assets avec vignettes et drag-and-drop.
  - Arbre de hiérarchie de scène et inspecteur de composants contextuel.

### Phase 4 : Level Design, Prefabs & Outils de Course Cyberpunk
- [x] Outil de placement avec raycast surface snapping et gizmos d'édition 3D.
- [x] Générateur de tunnels néons et circuits le long de splines 3D.
- [x] Intégration du système de triggers et d'événements visuels reliés aux scripts Rhai.

### Phase 5 : Mode Play-in-Editor (PIE) & Exporteur Autonome
- [x] Bouton Play/Stop dans le studio pour tester le gameplay instantanément avec réinitialisation de l'état ECS à l'arrêt.
- [x] Commande de packaging / build d'exécutable autonome propre.

---

## Vérification & Critères de Validation
* **Compilabilité** : `cargo check` et `cargo test` passent sans aucun avertissement.
* **Intégrité du DAG** : Vérification par tests unitaires que les graphes de nœuds cycliques sont détectés et que le code WGSL généré est syntaxiquement parfait.
* **Hot Reloading** : Les modifications de scripts Rhai et de graphes de matériaux se répercutent en <50ms dans le viewport 3D.
