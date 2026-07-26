-- wrk2 script for AEGIS sidecar latency testing
-- Usage: wrk2 -t2 -c10 -d30s -R1000 -s tests/benchmarks/wrk2-sidecar.lua http://localhost:9000/

wrk.method = "POST"
wrk.body = '{"model":"gpt-4","messages":[{"role":"user","content":"hello"}]}'
wrk.headers = {
    ["Content-Type"] = "application/json",
    ["X-AEGIS-Agent-ID"] = "bench-agent",
    ["X-AEGIS-Trace-ID"] = "bench-trace",
}

response = function(status, headers, body)
    if status ~= 200 and status ~= 403 then
        print("Unexpected status: " .. status)
    end
end

done = function(summary, latency, requests)
    io.write("\n--- Sidecar Latency Results ---\n")
    io.write(string.format("Requests/sec: %.0f\n", summary.requests / summary.duration))
    io.write(string.format("Total requests: %d\n", summary.requests))
    io.write(string.format("Duration: %.2f sec\n", summary.duration))
    io.write(string.format("Errors: %d\n", summary.errors.status + summary.errors.timeout + summary.errors.connect))
    io.write(string.format("P50: %.3f ms\n", latency:percentile(50) / 1000))
    io.write(string.format("P75: %.3f ms\n", latency:percentile(75) / 1000))
    io.write(string.format("P90: %.3f ms\n", latency:percentile(90) / 1000))
    io.write(string.format("P95: %.3f ms\n", latency:percentile(95) / 1000))
    io.write(string.format("P99: %.3f ms\n", latency:percentile(99) / 1000))
    io.write(string.format("Max: %.3f ms\n", latency.max / 1000))
    io.write("-----------------------------\n")
end
