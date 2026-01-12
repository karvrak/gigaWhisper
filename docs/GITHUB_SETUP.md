# Guide de Publication GitHub - GigaWhisper

Ce guide te permet de publier GigaWhisper sur GitHub avec CI/CD, issues et releases automatiques.

---

## Table des matières

1. [Prérequis](#prérequis)
2. [Étape 1 : Préparation locale](#étape-1--préparation-locale)
3. [Étape 2 : Création du dépôt GitHub](#étape-2--création-du-dépôt-github)
4. [Étape 3 : Liaison et premier push](#étape-3--liaison-et-premier-push)
5. [Étape 4 : Configuration GitHub](#étape-4--configuration-github)
6. [Étape 5 : Personnalisation des fichiers](#étape-5--personnalisation-des-fichiers)
7. [Étape 6 : Première release](#étape-6--première-release)
8. [Utilisation quotidienne](#utilisation-quotidienne)

---

## Prérequis

### Outils nécessaires

- **Git** installé sur ta machine
  ```bash
  # Vérifier l'installation
  git --version
  ```

- **GitHub CLI** (optionnel mais recommandé)
  ```bash
  # Installer avec winget
  winget install GitHub.cli

  # Ou télécharger depuis https://cli.github.com/
  ```

- **Compte GitHub** avec accès à https://github.com

### Authentification GitHub

```bash
# Avec GitHub CLI (recommandé)
gh auth login

# Ou configurer Git avec ton compte
git config --global user.name "Ton Nom"
git config --global user.email "ton.email@example.com"
```

---

## Étape 1 : Préparation locale

### 1.1 Ouvrir un terminal dans le projet

```bash
cd D:\VIBE-CODING\gigaWhisper
```

### 1.2 Initialiser le dépôt Git

```bash
# Initialiser Git
git init

# Vérifier que .gitignore existe (il devrait déjà être présent)
# Sinon, les fichiers sensibles seront exclus automatiquement
```

### 1.3 Premier commit

```bash
# Ajouter tous les fichiers
git add .

# Créer le premier commit
git commit -m "Initial commit: GigaWhisper v0.1.0

- Tauri 2 + React application for voice transcription
- Local (whisper.cpp) and cloud (Groq) transcription engines
- Global hotkeys, system tray integration
- CI/CD workflows for GitHub Actions"
```

---

## Étape 2 : Création du dépôt GitHub

### Option A : Avec GitHub CLI (rapide)

```bash
# Créer et pousser en une commande
gh repo create gigawhisper --public --source=. --remote=origin --push
```

C'est tout ! Passe directement à l'[Étape 4](#étape-4--configuration-github).

---

### Option B : Via l'interface web (manuel)

#### 2.1 Créer le dépôt

1. Va sur **https://github.com/new**

2. Remplis les informations :
   | Champ | Valeur |
   |-------|--------|
   | Repository name | `gigawhisper` |
   | Description | `Open-source voice transcription for Windows. A SuperWhisper alternative.` |
   | Visibility | **Public** |
   | Initialize with README | **Non** (décoché) |
   | Add .gitignore | **None** |
   | Choose a license | **None** (déjà incluse) |

3. Clique sur **"Create repository"**

#### 2.2 Copier l'URL du dépôt

Après création, GitHub affiche les instructions. Copie l'URL :
```
https://github.com/TON_USERNAME/gigawhisper.git
```

---

## Étape 3 : Liaison et premier push

### 3.1 Ajouter le remote

```bash
# Remplace TON_USERNAME par ton nom d'utilisateur GitHub
git remote add origin https://github.com/TON_USERNAME/gigawhisper.git

# Vérifier la liaison
git remote -v
```

### 3.2 Renommer la branche principale (si nécessaire)

```bash
# GitHub utilise "main" par défaut
git branch -M main
```

### 3.3 Pousser le code

```bash
# Premier push avec liaison de la branche
git push -u origin main
```

### 3.4 Vérification

Ouvre ton navigateur sur `https://github.com/TON_USERNAME/gigawhisper` et vérifie que :
- Tous les fichiers sont présents
- Le README s'affiche correctement
- Le dossier `.github` contient les workflows

---

## Étape 4 : Configuration GitHub

### 4.1 Activer GitHub Actions

1. Va dans **Settings** > **Actions** > **General**
2. Sous "Actions permissions", sélectionne **"Allow all actions"**
3. Sous "Workflow permissions", sélectionne **"Read and write permissions"**
4. Clique **Save**

### 4.2 Activer les Discussions (optionnel)

1. Va dans **Settings** > **General**
2. Scroll jusqu'à **Features**
3. Coche **"Discussions"**

### 4.3 Configurer les labels d'issues

1. Va dans **Issues** > **Labels**
2. Les labels par défaut sont créés automatiquement
3. Ajoute ces labels personnalisés (optionnel) :

| Label | Color | Description |
|-------|-------|-------------|
| `transcription` | `#7057ff` | Related to transcription engines |
| `audio` | `#008672` | Audio capture issues |
| `ui` | `#0075ca` | User interface |
| `hotkeys` | `#e4e669` | Keyboard shortcuts |

### 4.4 Protéger la branche main (recommandé)

1. Va dans **Settings** > **Branches**
2. Clique **"Add branch protection rule"**
3. Branch name pattern : `main`
4. Coche :
   - [x] Require a pull request before merging
   - [x] Require status checks to pass before merging
     - Sélectionne : `Frontend Lint & Build`, `Rust Check & Clippy`
5. Clique **"Create"**

---

## Étape 5 : Personnalisation des fichiers

### 5.1 Remplacer YOUR_USERNAME

Exécute cette commande pour remplacer automatiquement :

```bash
# PowerShell
$username = "TON_USERNAME"  # Remplace par ton username

# Remplacer dans les fichiers
(Get-Content .github/ISSUE_TEMPLATE/config.yml) -replace 'YOUR_USERNAME', $username | Set-Content .github/ISSUE_TEMPLATE/config.yml
(Get-Content CONTRIBUTING.md) -replace 'YOUR_USERNAME', $username | Set-Content CONTRIBUTING.md
```

### 5.2 Ajouter les badges au README

Ouvre `README.md` et ajoute ces lignes après le titre :

```markdown
# GigaWhisper

[![CI](https://github.com/TON_USERNAME/gigawhisper/actions/workflows/ci.yml/badge.svg)](https://github.com/TON_USERNAME/gigawhisper/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/TON_USERNAME/gigawhisper?include_prereleases)](https://github.com/TON_USERNAME/gigawhisper/releases)
[![License](https://img.shields.io/github/license/TON_USERNAME/gigawhisper)](LICENSE)

Open-source voice transcription...
```

### 5.3 Commit des modifications

```bash
git add .
git commit -m "chore: configure GitHub username in templates"
git push
```

---

## Étape 6 : Première release

### 6.1 Vérifier que le CI passe

1. Va dans **Actions** sur GitHub
2. Vérifie que le workflow "CI" est vert ✅
3. Si rouge, corrige les erreurs avant de continuer

### 6.2 Créer un tag de version

```bash
# Créer le tag
git tag -a v0.1.0 -m "Release v0.1.0 - Initial public release"

# Pousser le tag
git push origin v0.1.0
```

### 6.3 Suivre le build

1. Va dans **Actions**
2. Un workflow "Release" devrait démarrer automatiquement
3. Attend ~10-15 minutes pour le build Windows

### 6.4 Vérifier la release

1. Va dans **Releases** sur GitHub
2. Tu devrais voir `GigaWhisper v0.1.0`
3. Les fichiers disponibles :
   - `GigaWhisper_0.1.0_x64-setup.exe` (installateur NSIS)
   - `GigaWhisper_0.1.0_x64_en-US.msi` (installateur MSI)

### 6.5 Éditer les notes de release

1. Clique sur la release
2. Clique **"Edit"**
3. Ajoute une description des fonctionnalités :

```markdown
## Highlights

- 🎤 Voice transcription with global hotkey
- 🏠 Local transcription with whisper.cpp
- ☁️ Cloud transcription with Groq API
- ⌨️ Push-to-Talk and Toggle recording modes
- 📋 Auto-paste to active window

## Installation

Download the installer below and run it. GigaWhisper will be available in your Start menu.

## Requirements

- Windows 10/11 (64-bit)
- For local transcription: ~500MB disk space for models
```

---

## Utilisation quotidienne

### Workflow de développement

```bash
# 1. Créer une branche pour ta feature
git checkout -b feature/ma-feature

# 2. Faire tes modifications
# ...

# 3. Commit
git add .
git commit -m "feat: description de la feature"

# 4. Pousser la branche
git push -u origin feature/ma-feature

# 5. Créer une Pull Request sur GitHub
gh pr create --title "Ma feature" --body "Description"
```

### Créer une nouvelle release

```bash
# 1. S'assurer d'être sur main à jour
git checkout main
git pull

# 2. Mettre à jour les versions (3 fichiers)
# - package.json         → "version": "0.2.0"
# - src-tauri/Cargo.toml → version = "0.2.0"
# - src-tauri/tauri.conf.json → "version": "0.2.0"

# 3. Commit
git add .
git commit -m "chore: bump version to 0.2.0"
git push

# 4. Créer et pousser le tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

### Commandes utiles

```bash
# Voir le statut
git status

# Voir les branches
git branch -a

# Voir les tags
git tag -l

# Voir les logs
git log --oneline -10

# Annuler les modifications non committées
git checkout -- .

# Synchroniser avec le remote
git fetch --all --prune
```

---

## Résumé des URLs

| Resource | URL |
|----------|-----|
| Dépôt | `https://github.com/TON_USERNAME/gigawhisper` |
| Issues | `https://github.com/TON_USERNAME/gigawhisper/issues` |
| Pull Requests | `https://github.com/TON_USERNAME/gigawhisper/pulls` |
| Actions (CI/CD) | `https://github.com/TON_USERNAME/gigawhisper/actions` |
| Releases | `https://github.com/TON_USERNAME/gigawhisper/releases` |
| Discussions | `https://github.com/TON_USERNAME/gigawhisper/discussions` |

---

## Besoin d'aide ?

- **Documentation Git** : https://git-scm.com/doc
- **Documentation GitHub** : https://docs.github.com
- **GitHub CLI** : https://cli.github.com/manual/
