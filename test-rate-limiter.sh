#!/bin/bash

# レート制限テストスクリプト

ALGORITHMS=("token-bucket" "leaky-bucket" "fixed-window" "sliding-window-log" "sliding-window-counter")
PORTS=(3000 3001 3002)
LANGUAGES=("TypeScript" "Go" "Rust")

echo "=== Rate Limiter Test Script ==="
echo

test_algorithm() {
    local port=$1
    local lang=$2
    local algo=$3
    local count=$4
    
    echo "Testing $lang ($port) - $algo: $count requests"
    
    for i in $(seq 1 $count); do
        response=$(curl -s -w "\n%{http_code}" http://localhost:$port/api/data/$algo 2>/dev/null)
        status_code=$(echo "$response" | tail -n 1)
        
        if [ "$status_code" = "200" ]; then
            echo -n "✓"
        elif [ "$status_code" = "429" ]; then
            echo -n "✗"
        else
            echo -n "?"
        fi
    done
    echo
    
    # 統計情報を表示
    stats=$(curl -s http://localhost:$port/api/stats/$algo 2>/dev/null | jq -r '.stats | "Remaining: \(.remaining)/\(.limit)"' 2>/dev/null || echo "Stats unavailable")
    echo "  $stats"
    echo
}

# 各言語・アルゴリズムでテスト
for idx in ${!PORTS[@]}; do
    port=${PORTS[$idx]}
    lang=${LANGUAGES[$idx]}
    
    echo "### $lang Server (Port: $port) ###"
    echo
    
    # サーバーが起動しているか確認
    if ! curl -s http://localhost:$port/api/algorithms >/dev/null 2>&1; then
        echo "❌ $lang server not running on port $port"
        echo
        continue
    fi
    
    for algo in ${ALGORITHMS[@]}; do
        test_algorithm $port $lang $algo 15
        sleep 1
    done
done

echo "=== Test Complete ==="