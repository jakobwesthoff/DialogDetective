#!/bin/bash
# Mock dialog_detective script for VHS demo recording
# Simulates the actual output with realistic timing

# Accept arguments but ignore them (just for show)
# Usage: ./mock_dialog_detective.sh ./videos "The Flash" --season 5

echo "🔍 DialogDetective"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📺 Investigating: The Flash"
printf "📡 Fetching metadata... "
sleep 0.5
echo "✓ (1 seasons)"
printf "🔎 Scanning directory... "
sleep 0.3
echo "✓ (4 files)"
echo ""

# File 1
echo "🎬 [1/4] The_Flash_S05_t01.mkv"
printf "   ├─ Computing hash... "
sleep 0.8
echo "✓"
printf "   ├─ Extracting audio... "
sleep 1.2
echo "✓"
printf "   ├─ Transcribing... "
sleep 2
echo "✓ (en)"
printf "   └─ Matching episode... "
sleep 1.5
echo "✓"
echo ""

# File 2
echo "🎬 [2/4] The_Flash_S05_t02.mkv"
printf "   ├─ Computing hash... "
sleep 0.8
echo "✓"
printf "   ├─ Extracting audio... "
sleep 1.2
echo "✓"
printf "   ├─ Transcribing... "
sleep 2
echo "✓ (en)"
printf "   └─ Matching episode... "
sleep 1.5
echo "✓"
echo ""

# File 3
echo "🎬 [3/4] The_Flash_S05_t03.mkv"
printf "   ├─ Computing hash... "
sleep 0.8
echo "✓"
printf "   ├─ Extracting audio... "
sleep 1.2
echo "✓"
printf "   ├─ Transcribing... "
sleep 2
echo "✓ (en)"
printf "   └─ Matching episode... "
sleep 1.5
echo "✓"
echo ""

# File 4
echo "🎬 [4/4] The_Flash_S05_t04.mkv"
printf "   ├─ Computing hash... "
sleep 0.8
echo "✓"
printf "   ├─ Extracting audio... "
sleep 1.2
echo "✓"
printf "   ├─ Transcribing... "
sleep 2
echo "✓ (en)"
printf "   └─ Matching episode... "
sleep 1.5
echo "✓"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Dry Run - No files will be modified:"
echo ""
echo "  [RENAME] The_Flash_S05_t01.mkv → The Flash - S05E04 - News Flash.mkv"
echo "  [RENAME] The_Flash_S05_t02.mkv → The Flash - S05E03 - The Death of Vibe.mkv"
echo "  [RENAME] The_Flash_S05_t03.mkv → The Flash - S05E01 - Nora.mkv"
echo "  [RENAME] The_Flash_S05_t04.mkv → The Flash - S05E02 - Blocked.mkv"
echo ""
echo "💡 Use --mode rename or --mode copy to apply these changes"
