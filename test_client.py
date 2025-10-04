#!/usr/bin/env python3
"""
Comprehensive HTTP Testing Client for RustWeb Server

This client tests HTTP/1.1, HTTP/2, and HTTP/3 (when available) protocols
with various features including:
- Performance benchmarking
- Protocol comparison
- Security testing
- Load testing
- Feature validation
"""

import asyncio
import json
import ssl
import sys
import time
import urllib.parse
from typing import Dict, List, Optional, Tuple
import argparse
import statistics
from dataclasses import dataclass, asdict
from pathlib import Path

try:
    import httpx
    import h2.connection
    import h2.events
    import h2.config
    import socket
    import threading
    import requests
    from concurrent.futures import ThreadPoolExecutor, as_completed
except ImportError as e:
    print(f"Missing required dependency: {e}")
    print("Install with: pip install httpx h2 requests")
    sys.exit(1)

@dataclass
class TestResult:
    """Result of a single HTTP test"""
    protocol: str
    method: str
    url: str
    status_code: int
    response_time: float
    content_length: int
    headers: Dict[str, str]
    error: Optional[str] = None
    body_preview: Optional[str] = None

@dataclass
class BenchmarkResult:
    """Results of a benchmark test"""
    protocol: str
    total_requests: int
    successful_requests: int
    failed_requests: int
    total_time: float
    requests_per_second: float
    avg_response_time: float
    min_response_time: float
    max_response_time: float
    percentiles: Dict[str, float]

class RustWebTester:
    """Comprehensive tester for RustWeb server"""
    
    def __init__(self, base_url: str = "http://localhost:8080", 
                 https_url: str = "https://localhost:8443"):
        self.base_url = base_url
        self.https_url = https_url
        self.results: List[TestResult] = []
        
        # Create httpx client with custom SSL context for self-signed certs
        self.client = httpx.Client(verify=False, timeout=30.0)
        self.async_client = httpx.AsyncClient(verify=False, timeout=30.0)
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.client.close()
    
    async def __aenter__(self):
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.async_client.aclose()
    
    def test_http1(self, path: str = "/", method: str = "GET", **kwargs) -> TestResult:
        """Test HTTP/1.1 request"""
        url = f"{self.base_url}{path}"
        start_time = time.time()
        
        try:
            response = self.client.request(method, url, **kwargs)
            response_time = time.time() - start_time
            
            return TestResult(
                protocol="HTTP/1.1",
                method=method,
                url=url,
                status_code=response.status_code,
                response_time=response_time,
                content_length=len(response.content),
                headers=dict(response.headers),
                body_preview=response.text[:200] if response.text else None
            )
        except Exception as e:
            return TestResult(
                protocol="HTTP/1.1",
                method=method,
                url=url,
                status_code=0,
                response_time=time.time() - start_time,
                content_length=0,
                headers={},
                error=str(e)
            )
    
    def test_http2(self, path: str = "/", method: str = "GET", **kwargs) -> TestResult:
        """Test HTTP/2 request using httpx with HTTP/2 support"""
        url = f"{self.https_url}{path}"
        start_time = time.time()
        
        try:
            # Force HTTP/2 by using httpx with http2=True
            with httpx.Client(verify=False, http2=True, timeout=30.0) as client:
                response = client.request(method, url, **kwargs)
                response_time = time.time() - start_time
                
                # Check if HTTP/2 was actually used
                protocol = "HTTP/2" if hasattr(response, 'http_version') and response.http_version == "HTTP/2" else "HTTP/1.1 (fallback)"
                
                return TestResult(
                    protocol=protocol,
                    method=method,
                    url=url,
                    status_code=response.status_code,
                    response_time=response_time,
                    content_length=len(response.content),
                    headers=dict(response.headers),
                    body_preview=response.text[:200] if response.text else None
                )
        except Exception as e:
            return TestResult(
                protocol="HTTP/2",
                method=method,
                url=url,
                status_code=0,
                response_time=time.time() - start_time,
                content_length=0,
                headers={},
                error=str(e)
            )
    
    def test_http3(self, path: str = "/", method: str = "GET", **kwargs) -> TestResult:
        """Test HTTP/3 request (placeholder for when HTTP/3 is implemented)"""
        # HTTP/3 is not yet implemented in RustWeb, so this returns a placeholder
        return TestResult(
            protocol="HTTP/3",
            method=method,
            url=f"https://localhost:8443{path}",
            status_code=0,
            response_time=0.0,
            content_length=0,
            headers={},
            error="HTTP/3 not yet implemented in RustWeb"
        )
    
    def test_all_protocols(self, path: str = "/", method: str = "GET", **kwargs) -> List[TestResult]:
        """Test the same request across all supported protocols"""
        results = []
        
        print(f"Testing {method} {path} across all protocols...")
        
        # Test HTTP/1.1
        result_h1 = self.test_http1(path, method, **kwargs)
        results.append(result_h1)
        self.results.append(result_h1)
        print(f"  HTTP/1.1: {result_h1.status_code} ({result_h1.response_time:.3f}s)")
        
        # Test HTTP/2
        result_h2 = self.test_http2(path, method, **kwargs)
        results.append(result_h2)
        self.results.append(result_h2)
        print(f"  HTTP/2: {result_h2.status_code} ({result_h2.response_time:.3f}s)")
        
        # Test HTTP/3 (placeholder)
        result_h3 = self.test_http3(path, method, **kwargs)
        results.append(result_h3)
        self.results.append(result_h3)
        print(f"  HTTP/3: Not implemented")
        
        return results
    
    def benchmark_protocol(self, protocol: str, num_requests: int = 100, 
                          path: str = "/", concurrent: int = 10) -> BenchmarkResult:
        """Benchmark a specific protocol"""
        print(f"\nBenchmarking {protocol} with {num_requests} requests ({concurrent} concurrent)...")
        
        results = []
        start_time = time.time()
        
        def make_request():
            if protocol == "HTTP/1.1":
                return self.test_http1(path)
            elif protocol == "HTTP/2":
                return self.test_http2(path)
            elif protocol == "HTTP/3":
                return self.test_http3(path)
            else:
                raise ValueError(f"Unknown protocol: {protocol}")
        
        # Use ThreadPoolExecutor for concurrent requests
        with ThreadPoolExecutor(max_workers=concurrent) as executor:
            futures = [executor.submit(make_request) for _ in range(num_requests)]
            
            for future in as_completed(futures):
                try:
                    result = future.result()
                    results.append(result)
                except Exception as e:
                    results.append(TestResult(
                        protocol=protocol,
                        method="GET",
                        url=path,
                        status_code=0,
                        response_time=0,
                        content_length=0,
                        headers={},
                        error=str(e)
                    ))
        
        total_time = time.time() - start_time
        successful = [r for r in results if r.status_code >= 200 and r.status_code < 400]
        failed = [r for r in results if r.status_code == 0 or r.status_code >= 400]
        
        if successful:
            response_times = [r.response_time for r in successful]
            percentiles = {
                "50": statistics.quantiles(response_times, n=2)[0],
                "90": statistics.quantiles(response_times, n=10)[8],
                "95": statistics.quantiles(response_times, n=20)[18],
                "99": statistics.quantiles(response_times, n=100)[98],
            }
        else:
            response_times = []
            percentiles = {}
        
        return BenchmarkResult(
            protocol=protocol,
            total_requests=num_requests,
            successful_requests=len(successful),
            failed_requests=len(failed),
            total_time=total_time,
            requests_per_second=len(successful) / total_time if total_time > 0 else 0,
            avg_response_time=statistics.mean(response_times) if response_times else 0,
            min_response_time=min(response_times) if response_times else 0,
            max_response_time=max(response_times) if response_times else 0,
            percentiles=percentiles
        )
    
    def test_security_features(self) -> List[TestResult]:
        """Test security features like rate limiting, headers, etc."""
        print("\nTesting security features...")
        security_results = []
        
        # Test rate limiting by making many requests quickly
        print("  Testing rate limiting...")
        rate_limit_results = []
        for i in range(10):
            result = self.test_http1("/", "GET")
            rate_limit_results.append(result)
            if result.status_code == 429:  # Too Many Requests
                print(f"    Rate limiting activated after {i+1} requests")
                break
            time.sleep(0.1)  # Small delay between requests
        
        security_results.extend(rate_limit_results)
        
        # Test security headers
        print("  Testing security headers...")
        result = self.test_http1("/")
        if result.headers:
            security_headers = [
                'x-frame-options',
                'x-content-type-options', 
                'x-xss-protection',
                'strict-transport-security'
            ]
            found_headers = []
            for header in security_headers:
                if header in result.headers:
                    found_headers.append(header)
            print(f"    Found security headers: {found_headers}")
        
        # Test different HTTP methods
        print("  Testing HTTP methods...")
        methods = ["GET", "POST", "HEAD", "PUT", "DELETE", "OPTIONS"]
        for method in methods:
            result = self.test_http1("/", method)
            security_results.append(result)
            print(f"    {method}: {result.status_code}")
        
        return security_results
    
    def test_server_features(self) -> List[TestResult]:
        """Test various server features"""
        print("\nTesting server features...")
        feature_results = []
        
        # Test different paths
        paths = [
            "/",
            "/index.html",
            "/api/test",
            "/static/test.css",
            "/nonexistent",
        ]
        
        for path in paths:
            results = self.test_all_protocols(path)
            feature_results.extend(results)
        
        # Test compression by requesting large content
        print("\nTesting compression...")
        result = self.test_http1("/", headers={"Accept-Encoding": "gzip, deflate, br"})
        if 'content-encoding' in result.headers:
            print(f"  Content encoding: {result.headers['content-encoding']}")
        feature_results.append(result)
        
        return feature_results
    
    def print_results_summary(self):
        """Print a summary of all test results"""
        print("\n" + "="*60)
        print("TEST RESULTS SUMMARY")
        print("="*60)
        
        if not self.results:
            print("No test results available")
            return
        
        # Group by protocol
        by_protocol = {}
        for result in self.results:
            if result.protocol not in by_protocol:
                by_protocol[result.protocol] = []
            by_protocol[result.protocol].append(result)
        
        for protocol, results in by_protocol.items():
            successful = [r for r in results if r.status_code >= 200 and r.status_code < 400]
            failed = [r for r in results if r.status_code == 0 or r.status_code >= 400]
            
            print(f"\n{protocol}:")
            print(f"  Total requests: {len(results)}")
            print(f"  Successful: {len(successful)}")
            print(f"  Failed: {len(failed)}")
            
            if successful:
                times = [r.response_time for r in successful]
                print(f"  Average response time: {statistics.mean(times):.3f}s")
                print(f"  Min response time: {min(times):.3f}s")
                print(f"  Max response time: {max(times):.3f}s")
        
        # Show some example responses
        print(f"\nExample successful responses:")
        successful_results = [r for r in self.results if r.status_code >= 200 and r.status_code < 400]
        for result in successful_results[:3]:
            print(f"  {result.protocol} {result.method} {result.url}")
            print(f"    Status: {result.status_code}, Time: {result.response_time:.3f}s")
            if result.body_preview:
                print(f"    Body: {result.body_preview[:100]}...")
    
    def save_results_json(self, filename: str = "test_results.json"):
        """Save test results to JSON file"""
        data = {
            "test_timestamp": time.time(),
            "base_url": self.base_url,
            "https_url": self.https_url,
            "results": [asdict(result) for result in self.results]
        }
        
        Path(filename).write_text(json.dumps(data, indent=2))
        print(f"\nResults saved to {filename}")

def main():
    parser = argparse.ArgumentParser(description="Test RustWeb HTTP server")
    parser.add_argument("--base-url", default="http://localhost:8080", 
                       help="Base HTTP URL (default: http://localhost:8080)")
    parser.add_argument("--https-url", default="https://localhost:8443",
                       help="HTTPS URL (default: https://localhost:8443)")
    parser.add_argument("--benchmark", action="store_true",
                       help="Run performance benchmarks")
    parser.add_argument("--num-requests", type=int, default=100,
                       help="Number of requests for benchmark (default: 100)")
    parser.add_argument("--concurrent", type=int, default=10,
                       help="Concurrent requests for benchmark (default: 10)")
    parser.add_argument("--security", action="store_true",
                       help="Run security tests")
    parser.add_argument("--save-json", 
                       help="Save results to JSON file")
    
    args = parser.parse_args()
    
    print("RustWeb HTTP Server Tester")
    print("=" * 40)
    print(f"Testing HTTP server at: {args.base_url}")
    print(f"Testing HTTPS server at: {args.https_url}")
    
    with RustWebTester(args.base_url, args.https_url) as tester:
        # Basic functionality tests
        print("\n1. Basic Protocol Tests")
        tester.test_all_protocols("/")
        
        # Feature tests
        print("\n2. Server Feature Tests")
        tester.test_server_features()
        
        # Security tests
        if args.security:
            print("\n3. Security Tests")
            tester.test_security_features()
        
        # Performance benchmarks
        if args.benchmark:
            print("\n4. Performance Benchmarks")
            protocols = ["HTTP/1.1", "HTTP/2"]  # HTTP/3 when available
            
            for protocol in protocols:
                try:
                    benchmark = tester.benchmark_protocol(
                        protocol, 
                        args.num_requests, 
                        concurrent=args.concurrent
                    )
                    
                    print(f"\n{protocol} Benchmark Results:")
                    print(f"  Successful requests: {benchmark.successful_requests}/{benchmark.total_requests}")
                    print(f"  Requests per second: {benchmark.requests_per_second:.2f}")
                    print(f"  Average response time: {benchmark.avg_response_time:.3f}s")
                    print(f"  Response time percentiles:")
                    for p, value in benchmark.percentiles.items():
                        print(f"    {p}th: {value:.3f}s")
                        
                except Exception as e:
                    print(f"  Benchmark failed for {protocol}: {e}")
        
        # Print summary
        tester.print_results_summary()
        
        # Save results if requested
        if args.save_json:
            tester.save_results_json(args.save_json)

if __name__ == "__main__":
    main()