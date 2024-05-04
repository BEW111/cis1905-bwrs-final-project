#!/bin/bash

# Define the URL of the endpoint
URL="http://127.0.0.1:8080/upload"

# Specify the path to your JSON file
JSON_FILE="wikipedia.json"

# Check if the JSON file exists
if [[ ! -f "$JSON_FILE" ]]; then
    echo "Error: JSON file does not exist."
    exit 1
fi

# Parse the JSON file and read each 'text' field
jq -c '.[] | {content: .text}' "$JSON_FILE" | while read -r payload
do
    # Use curl to send a POST request with the text as the document content
    curl -X POST "$URL" \
        -H "Content-Type: application/json" \
        -d "{\"content\": \"$text\"}"
    echo ""  # Print a newline for better readability in output
done
