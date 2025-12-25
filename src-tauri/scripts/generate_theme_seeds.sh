#!/bin/bash
# Generate SQL seed migration from theme JSON files

echo "-- Auto-generated migration to seed built-in themes" > ../migrations/002_seed_builtin_themes.sql
echo "" >> ../migrations/002_seed_builtin_themes.sql

# Process each JSON file
for theme_file in ../themes/*.json; do
    if [ ! -f "$theme_file" ]; then
        continue
    fi

    echo "Processing $theme_file..."

    # Extract values using jq (if available) or basic parsing
    if command -v jq &> /dev/null; then
        # Use jq for proper JSON parsing
        id=$(jq -r '.id' "$theme_file")
        name=$(jq -r '.name' "$theme_file")
        author=$(jq -r '.author // "Tables IDE"' "$theme_file")
        description=$(jq -r '.description // "No description"' "$theme_file")
        theme_data=$(jq -c '.' "$theme_file")
    else
        # Fallback to basic sed/awk parsing (less reliable)
        id=$(grep '"id"' "$theme_file" | head -1 | sed 's/.*"id":\s*"\([^"]*\)".*/\1/')
        name=$(grep '"name"' "$theme_file" | head -1 | sed 's/.*"name":\s*"\([^"]*\)".*/\1/')
        author=$(grep '"author"' "$theme_file" | head -1 | sed 's/.*"author":\s*"\([^"]*\)".*/\1/' || echo "Tables IDE")
        description=$(grep '"description"' "$theme_file" | head -1 | sed 's/.*"description":\s*"\([^"]*\)".*/\1/' || echo "No description")
        theme_data=$(cat "$theme_file" | tr -d '\n\t' | sed 's/"/""/g')
    fi

    # Escape single quotes for SQL
    id="${id//\'/\'\'}"
    name="${name//\'/\'\'}"
    author="${author//\'/\'\'}"
    description="${description//\'/\'\'}"
    theme_data="${theme_data//\'/\'\'}"

    # Generate INSERT statement
    echo "INSERT OR REPLACE INTO themes (id, name, author, description, theme_data, is_builtin, is_active) VALUES" >> ../migrations/002_seed_builtin_themes.sql
    echo "  ('$id', '$name', '$author', '$description', '$theme_data', 1, 0);" >> ../migrations/002_seed_builtin_themes.sql
    echo "" >> ../migrations/002_seed_builtin_themes.sql
done

# Set Monokai as default active theme
echo "-- Set Monokai as the default active theme" >> ../migrations/002_seed_builtin_themes.sql
echo "UPDATE themes SET is_active = 1 WHERE id = 'monokai';" >> ../migrations/002_seed_builtin_themes.sql

echo "Migration file generated: src-tauri/migrations/002_seed_builtin_themes.sql"