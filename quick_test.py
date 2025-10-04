#!/usr/bin/env python3
"""Quick test to demonstrate specific RustWeb features"""

import requests
import time
import concurrent.futures

# Disable SSL warnings for self-signed certs
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

def test_rate_limiting():
    """Test rate limiting by making rapid requests"""
    print("Testing rate limiting (sending 120 requests quickly)...")
    
    responses = []
    start_time = time.time()
    
    # Send requests rapidly to trigger rate limiting
    with concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        futures = []
        for i in range(120):
            future = executor.submit(requests.get, "http://localhost:8080/", timeout=5)
            futures.append(future)
        
        for future in concurrent.futures.as_completed(futures):
            try:
                response = future.result()
                responses.append(response.status_code)
            except Exception as e:
                responses.append(0)
    
    elapsed = time.time() - start_time
    success_count = sum(1 for code in responses if code == 200)
    rate_limited_count = sum(1 for code in responses if code == 429)
    
    print(f"Results after {elapsed:.2f} seconds:")
    print(f"  Total requests: {len(responses)}")
    print(f"  Successful (200): {success_count}")
    print(f"  Rate limited (429): {rate_limited_count}")
    print(f"  Other errors: {len(responses) - success_count - rate_limited_count}")
    print(f"  Requests per second: {len(responses) / elapsed:.2f}")

def test_protocols():
    """Test different protocols and features"""
    print("\nTesting protocols and features...")
    
    # Test HTTP/1.1
    print("HTTP/1.1 test:")
    resp = requests.get("http://localhost:8080/")
    print(f"  Status: {resp.status_code}, Content: {resp.text[:50]}...")
    
    # Test HTTP/2 (over HTTPS)
    print("HTTP/2 test:")
    resp = requests.get("https://localhost:8443/", verify=False)
    print(f"  Status: {resp.status_code}, Content: {resp.text[:50]}...")
    
    # Test security headers
    print("Security headers:")
    for header in ['x-frame-options', 'x-content-type-options', 'server']:
        if header in resp.headers:
            print(f"  {header}: {resp.headers[header]}")

def test_methods():
    """Test different HTTP methods"""
    print("\nTesting HTTP methods:")
    methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']
    
    for method in methods:
        try:
            resp = requests.request(method, "http://localhost:8080/", timeout=3)
            print(f"  {method}: {resp.status_code}")
        except Exception as e:
            print(f"  {method}: Error - {e}")

if __name__ == "__main__":
    test_protocols()
    test_methods()
    test_rate_limiting()