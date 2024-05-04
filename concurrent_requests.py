import requests
from concurrent.futures import ThreadPoolExecutor

def send_request():
    url = 'http://127.0.0.1:3535/api/upload_document'
    data = {'content': 'Horses'}  
    response = requests.post(url, json=data)
    print(response.text)

with ThreadPoolExecutor(max_workers=10) as executor:
    futures = [executor.submit(send_request) for _ in range(100)]  # Send 100 concurrent requests
