"""OpenAI Agents SDK integration with AEGIS sidecar.

Usage:
    pip install openai requests
    python aegis_wrapper.py
"""

import os
import requests
from openai import OpenAI

SIDECAR_URL = os.getenv("AEGIS_SIDECAR_URL", "http://localhost:9000")
client = OpenAI()


def aegis_govern(operation: str, resource: str) -> dict:
    agent_id = os.getenv("AEGIS_AGENT_ID", "openai-agent")
    resp = requests.post(
        f"{SIDECAR_URL}/v1/govern",
        json={
            "operation": operation,
            "resource": resource,
            "agent_id": agent_id,
            "environment": os.getenv("AEGIS_ENVIRONMENT", "production"),
        },
        headers={
            "X-AEGIS-Agent-ID": agent_id,
        },
    )
    return resp.json() if resp.status_code == 200 else {"decision": "DENY"}


def governed_chat_completion(messages, model="gpt-4", **kwargs):
    decision = aegis_govern("llm.invoke", model)
    if decision.get("decision") == "DENY":
        raise PermissionError(f"LLM call denied: {decision.get('reason', 'policy violation')}")
    return client.chat.completions.create(model=model, messages=messages, **kwargs)


def governed_tool_call(tool_name: str, arguments: dict):
    decision = aegis_govern("tool.call", tool_name)
    if decision.get("decision") == "DENY":
        raise PermissionError(f"Tool call denied: {decision.get('reason', 'policy violation')}")
    return {"tool": tool_name, "result": f"Executed with {arguments}"}


if __name__ == "__main__":
    response = governed_chat_completion(
        messages=[{"role": "user", "content": "What is the capital of France?"}]
    )
    print(response.choices[0].message.content)
