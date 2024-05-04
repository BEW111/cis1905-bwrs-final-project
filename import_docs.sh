#!/bin/bash

# Define the URL of the endpoint
# URL="http://127.0.0.1:8080/upload_old"
URL="http://127.0.0.1:8080/upload"

# Specify the path to your JSON file
JSON_FILE="wikipedia.json"

# Check if the JSON file exists
if [[ ! -f "$JSON_FILE" ]]; then
    echo "Error: JSON file does not exist."
    exit 1
fi

# Parse the JSON file and read each 'text' field, formatting it as JSON
# Initialize a counter
counter=0
max_iterations=100  # Set the maximum number of iterations

jq -c '.[] | {content: .text}' "$JSON_FILE" | while read -r payload
do
    # Increment the counter
    ((counter++))

    # Use curl to send a POST request with the text as the document content
    curl -X POST "$URL" \
        -H "Content-Type: application/json" \
        -d "$payload"
    echo ""  # Print a newline for better readability in output

    # Break the loop if the counter reaches the maximum number of iterations
    if [[ $counter -eq $max_iterations ]]; then
        break
    fi
done
