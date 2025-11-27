#!/bin/bash

# Image Difference Visualization Script
# Compares two images and highlights the differences

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <image1> <image2>"
    echo "Example: $0 screenshot1.png screenshot2.png"
    exit 1
fi

IMAGE1="$1"
IMAGE2="$2"

# Проверяем существование файлов
if [ ! -f "$IMAGE1" ]; then
    echo "Error: File '$IMAGE1' not found!"
    exit 1
fi

if [ ! -f "$IMAGE2" ]; then
    echo "Error: File '$IMAGE2' not found!"
    exit 1
fi

echo "=== Image Difference Analyzer ==="
echo ""
echo "Image 1: $IMAGE1"
echo "Image 2: $IMAGE2"
echo ""

# Создаём имя для результата
OUTPUT_DIR=$(dirname "$IMAGE1")
BASENAME1=$(basename "$IMAGE1" | sed 's/\.[^.]*$//')
BASENAME2=$(basename "$IMAGE2" | sed 's/\.[^.]*$//')
OUTPUT_DIFF="${OUTPUT_DIR}/diff_${BASENAME1}_vs_${BASENAME2}.png"
OUTPUT_HIGHLIGHT="${OUTPUT_DIR}/highlight_${BASENAME1}_vs_${BASENAME2}.png"
OUTPUT_SIDE="${OUTPUT_DIR}/sidebyside_${BASENAME1}_vs_${BASENAME2}.png"

echo "Analyzing differences..."
echo ""

# 1. Создаём изображение различий (красным цветом)
compare -highlight-color red -lowlight-color white "$IMAGE1" "$IMAGE2" "$OUTPUT_DIFF" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✓ Difference map created: $OUTPUT_DIFF"
else
    echo "✗ Failed to create difference map"
fi

# 2. Создаём изображение с выделением изменённых областей
compare -compose src "$IMAGE1" "$IMAGE2" -compose over -highlight-color red "$OUTPUT_HIGHLIGHT" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✓ Highlighted differences: $OUTPUT_HIGHLIGHT"
else
    echo "✗ Failed to create highlighted version"
fi

# 3. Создаём сравнение side-by-side с линией разделения
convert "$IMAGE1" "$IMAGE2" +append "$OUTPUT_SIDE" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✓ Side-by-side comparison: $OUTPUT_SIDE"
else
    echo "✗ Failed to create side-by-side comparison"
fi

# 4. Вычисляем процент различий
METRIC=$(compare -metric AE "$IMAGE1" "$IMAGE2" null: 2>&1)
SIZE1=$(identify -format "%wx%h" "$IMAGE1" 2>/dev/null)
TOTAL_PIXELS=$(echo $SIZE1 | awk -F'x' '{print $1 * $2}')

if [ ! -z "$TOTAL_PIXELS" ] && [ "$TOTAL_PIXELS" -gt 0 ]; then
    DIFF_PERCENT=$(echo "scale=2; ($METRIC / $TOTAL_PIXELS) * 100" | bc)
    echo ""
    echo "=== Statistics ==="
    echo "Image size: $SIZE1"
    echo "Different pixels: $METRIC"
    echo "Difference: $DIFF_PERCENT%"
fi

# 5. Создаём комбинированное изображение (оригинал + различия)
convert \( "$IMAGE1" "$OUTPUT_DIFF" +append \) \
        \( "$IMAGE2" "$OUTPUT_HIGHLIGHT" +append \) \
        -append "${OUTPUT_DIR}/combined_analysis.png" 2>/dev/null

if [ $? -eq 0 ]; then
    echo ""
    echo "✓ Combined analysis: ${OUTPUT_DIR}/combined_analysis.png"
fi

echo ""
echo "=== Output Files ==="
echo "1. Difference map (grayscale):     $OUTPUT_DIFF"
echo "2. Highlighted changes (red):      $OUTPUT_HIGHLIGHT"
echo "3. Side-by-side comparison:        $OUTPUT_SIDE"
echo "4. Combined analysis (4-panel):    ${OUTPUT_DIR}/combined_analysis.png"
echo ""
echo "Done! Check the output files to see the differences."
