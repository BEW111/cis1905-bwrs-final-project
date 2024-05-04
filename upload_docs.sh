#!/bin/bash

# Define the URL of the endpoint
URL="http://127.0.0.1:8080/upload"

# Specify the path to your text file
FILE_PATH="docs.txt"

# Check if the file exists
if [[ ! -f "$FILE_PATH" ]]; then
    echo "Error: File does not exist."
    exit 1
fi

# Read each line from the file
while IFS= read -r line
do
    # Use curl to send a POST request for each line read from the file
    curl -X POST "$URL" \
        -H "Content-Type: application/json" \
        -d "{\"content\": \"$line\"}"
    echo ""  # Print a newline for better readability in output
done < "$FILE_PATH"
