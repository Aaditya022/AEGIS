"""LangGraph integration with AEGIS sidecar.

Usage:
    pip install langgraph requests
    python aegis_sidecar.py
"""

import os
import json
import requests
from typing import Any, Callable
from langgraph.graph import StateGraph, END

SIDECAR_URL = os.getenv("AEGIS_SIDECAR_URL", "http://localhost:9000")


def aegis_govern(operation: str, resource: str, agent_id: str) -> dict:
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
            "X-AEGIS-Trace-ID": os.urandom(16).hex(),
        },
    )
    return resp.json() if resp.ok else {"decision": "DENY", "reason": "sidecar error"}


def aegis_check_tool(tool: str, args: dict, agent_id: str) -> bool:
    resp = requests.post(
        f"{SIDECAR_URL}/v1/check-tool",
        json={"tool": tool, "target": args.get("target", ""), "agent_id": agent_id},
    )
    return resp.status_code == 200


def create_governed_agent(agent_id: str) -> StateGraph:
    graph = StateGraph(dict)

    def call_llm(state):
        decision = aegis_govern("llm.invoke", "gpt-4", agent_id)
        if decision.get("decision") == "DENY":
            return {"error": f"Governance denied: {decision.get('reason')}"}
        return {"result": "LLM call permitted"}

    def call_tool(state):
        tool = state.get("tool", "")
        args = state.get("args", {})
        if not aegis_check_tool(tool, args, agent_id):
            return {"error": f"Tool {tool} blocked by governance"}
        return {"result": f"Tool {tool} executed"}

    graph.add_node("call_llm", call_llm)
    graph.add_node("call_tool", call_tool)
    graph.set_entry_point("call_llm")
    graph.add_edge("call_llm", "call_tool")
    graph.add_edge("call_tool", END)

    return graph.compile()


if __name__ == "__main__":
    agent = create_governed_agent("spiffe://aegis.local/ns/default/sa/my-agent")
    result = agent.invoke({"tool": "search_documents", "args": {"query": "reports"}})
    print(json.dumps(result, indent=2))
