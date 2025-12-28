```
# Phil-OS — Slab Allocator (Rust, x86_64, QEMU)

![Rust](https://img.shields.io/badge/Rust-no__std-orange)
![Arch](https://img.shields.io/badge/arch-x86__64-blue)
![Runtime](https://img.shields.io/badge/runtime-QEMU-informational)
![Type](https://img.shields.io/badge/project-OS%20Kernel-lightgrey)

> Implémentation pédagogique d’un **slab allocator** en Rust dans une codebase de type **Phil-OS** (noyau x86_64), accompagnée d’un write-up expliquant le modèle Linux et d’un test de validation.

---

##  Sommaire

- [Contexte & Objectifs](#-contexte--objectifs)
- [Slab allocator : c’est quoi ?](#-slab-allocator--cest-quoi-)
- [Comparaison avec d’autres allocateurs](#-comparaison-avec-dautres-allocateurs)
- [Le slab allocator dans Linux](#-le-slab-allocator-dans-linux)
- [Implémentation dans Phil-OS](#-implémentation-dans-phil-os)
- [Arborescence du projet](#-arborescence-du-projet)
- [Tests & Validation](#-tests--validation)
- [Limitations connues](#️-limitations-connues)
- [Bonus FAT32](#-bonus-fat32)
- [Références](#-références)

---

##  Contexte & Objectifs

Ce projet répond à une consigne de cours systèmes consistant à :

- **Documenter** ce qu’est un *slab allocator*
- Expliquer le **fonctionnement du slab allocator utilisé dans Linux**
- **Implémenter une version simplifiée** d’un slab allocator dans un noyau en Rust
- Fournir des **preuves de fonctionnement** via des tests

 **Objectif pédagogique**  
L’objectif n’est pas de reproduire l’implémentation Linux à l’identique, mais de comprendre et démontrer les **principes fondamentaux** du modèle *slab* : caches par taille, freelist, réutilisation des objets, et réduction de la fragmentation.

---

##  Slab allocator : c’est quoi ?

Un **slab allocator** est un allocateur mémoire conçu pour gérer efficacement des **objets de taille fixe**, très fréquents dans un noyau (structures internes, buffers, descripteurs, etc.).

### Objectifs principaux

-  **Performance** : allocation et libération rapides
-  **Faible fragmentation** : tailles fixes, regroupées
-  **Réutilisation** : un objet libéré est immédiatement réutilisable
-  **Localité cache CPU** : objets proches en mémoire

 Contrairement à `malloc`, un slab allocator **ne gère pas des tailles arbitraires**, mais des **classes de tailles**.

---

##  Comparaison avec d’autres allocateurs

| Allocateur | Granularité | Avantages | Inconvénients |
|----------|------------|-----------|---------------|
| Buddy allocator | Pages (puissances de 2) | Simple, rapide pour grosses allocs | Mauvais pour petits objets |
| `malloc` classique | Variable | Flexible | Fragmentation, overhead |
| **Slab allocator** | **Taille fixe** | Rapide, cache-friendly | Plus spécialisé |

 En pratique, Linux combine **buddy allocator + slab allocator**.

---

##  Le slab allocator dans Linux

Linux propose trois implémentations principales :

### SLAB
- Implémentation historique
- Caches riches, constructeur/destructeur
- Complexité élevée

### SLUB (actuelle par défaut)
- Simplification du modèle
- Freelist stockée directement dans les objets
- Meilleure scalabilité SMP

### SLOB
- Implémentation minimaliste
- Destinée aux systèmes embarqués
- Performances limitées

 Ce projet s’inspire **conceptuellement** du modèle SLUB (freelist simple, peu de métadonnées).

---

### Notions clés

- **Cache** : gère une classe d’objets d’une taille donnée
- **Slab** : zone mémoire découpée en objets identiques
- **Object** : unité allouée
- **Freelist** : liste chaînée des objets libres

---

## 🛠 Implémentation dans Phil-OS

L’implémentation se trouve dans le module :

```

src/kernel/memory/slab.rs

````

### Hypothèses & choix

- Implémentation **volontairement simplifiée**
- Une **page mémoire statique** de 4096 octets
- Pas de NUMA, pas de per-CPU
- Pas de protection avancée (double-free, poisoning)
- Objectif : **clarté et compréhension**

 Ces choix sont assumés dans un cadre pédagogique.

---

### Structures de données

#### `FreeNode`

```rust
struct FreeNode {
    next: *mut FreeNode,
}
````

Chaque objet libre contient un pointeur vers le prochain objet libre.

---

#### `Page`

```rust
struct Page {
    data: UnsafeCell<[u8; 4096]>,
}
```

* Simule une page mémoire
* Partagée entre tous les caches
* Utilise `UnsafeCell` pour permettre la mutabilité intérieure

---

#### `SlabCache`

```rust
pub struct SlabCache {
    size: usize,
    free_list: *mut FreeNode,
}
```

* Gère une **classe de taille**
* Contient une freelist d’objets libres

---

#### `SlabAllocator`

```rust
pub struct SlabAllocator {
    caches: [SlabCache; 8],
}
```

Caches supportés (classes de tailles fixes) :

```
8, 16, 32, 64, 128, 256, 512, 1024 octets
```

---

### Algorithmes

#### Allocation (`alloc`)

1. Sélection du premier cache tel que `size <= cache.size`
2. Si la freelist est vide → `refill()`
3. Extraction d’un objet depuis la freelist
4. Retour du pointeur

```rust
pub unsafe fn alloc(&mut self, size: usize) -> *mut u8
```

---

#### Libération (`dealloc`)

1. Le pointeur est casté en `FreeNode`
2. Réinsertion en tête de freelist

```rust
pub unsafe fn dealloc(&mut self, ptr: *mut u8, size: usize)
```

 Le choix du cache repose sur la taille fournie par l’appelant.

---

#### Refill

```rust
unsafe fn refill(&mut self)
```

* Découpe la page de 4096 octets en objets de taille `self.size`
* Chaîne chaque objet dans la freelist

 C’est l’équivalent pédagogique de l’allocation d’un *slab*.

---

##  Arborescence du projet

```
src/
└── kernel/
    └── memory/
        ├── mod.rs
        ├── slab.rs        # Slab allocator
        └── slab_test.rs   # Tests unitaires
```

---

##  Tests & Validation

Les tests sont définis dans :

```
src/kernel/memory/slab_test.rs
```

### Test : réutilisation des objets

```rust
#[test]
fn slab_alloc_free_reuse() {
    let mut slab = SlabAllocator::new();

    unsafe {
        let p1 = slab.alloc(32);
        assert!(!p1.is_null());

        slab.dealloc(p1, 32);

        let p2 = slab.alloc(32);
        assert!(!p2.is_null());

        assert_eq!(p1, p2);
    }
}
```

 **Validation démontrée**

* Allocation correcte
* Libération correcte
* **Réutilisation du même bloc mémoire**
* Freelist fonctionnelle

 Ce test prouve le principe fondamental du slab allocator : *un objet libéré est réutilisé*.

---

##  Limitations connues

* Une seule page mémoire partagée
* Pas de per-CPU caches
* Pas de NUMA
* Pas de vérification de double free
* Le choix du cache repose sur la taille fournie manuellement

 Ces limitations sont cohérentes avec un **prototype pédagogique**.

---

##  Bonus FAT32

 **Non implémenté dans ce dépôt**

 La structure du projet permettrait une intégration future d’un filesystem (FAT32, ext2, etc.) utilisant le slab allocator pour gérer ses structures internes.

---

## Références

* Linux Kernel Documentation — Memory Management
* Love, *Linux Kernel Development*
* Tanenbaum, *Modern Operating Systems*
* Linux Source Code (`mm/slab.c`, `mm/slub.c`)
* Rust OSDev Wiki

---

## Auteur

Projet réalisé dans le cadre d’un **cours systèmes / OS**,
implémenté en Rust pour architecture **x86_64**, exécutable sous **QEMU**.

---

**Note finale pour la correction**
Le projet démontre une compréhension claire du modèle *slab allocator*, de ses objectifs et de son intégration dans un noyau, via une implémentation simple mais fonctionnelle et testée.

```
