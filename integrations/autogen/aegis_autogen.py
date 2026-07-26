"""AutoGen integration with AEGIS sidecar.

Usage:
    pip install pyautogen requests
    python aegis_autogen.py
"""

import os
import requests
from autogen import AssistantAgent, UserProxyAgent

SIDECAR_URL = os.getenv("AEGIS_SIDECAR_URL", "http://localhost:9000")
AGENT_ID = os.getenv("AEGIS_AGENT_ID", "autogen-agent")


def aegis_govern(operation: str, resource: str) -> bool:
    resp = requests.post(
        f"{SIDECAR_URL}/v1/govern",
        json={
            "operation": operation,
            "resource": resource,
            "agent_id": AGENT_ID,
            "environment": os.getenv("AEGIS_ENVIRONMENT", "production"),
        },
    )
    return resp.status_code != 403


class AEGISAssistantAgent(AssistantAgent):
    def __init__(self, name, llm_config=None, **kwargs):
        super().__init__(name, llm_config=llm_config, **kwargs)
        self.register_reply([AssistantAgent, None], self.aegis_reply)

    def aegis_reply(self, messages=None, sender=None, config=None):
        for msg in (messages or []):
            if "function_call" in msg:
                func_name = msg["function_call"].get("name", "")
                if not aegis_govern("tool.call", func_name):
                    return True, {
                        "role": "function",
                        "name": func_name,
                        "content": f"ERROR: Tool '{func_name}' blocked by AEGIS governance",
                    }
        return False, None


assistant = AEGISAssistantAgent(
    name="aegis_assistant",
    llm_config={
        "config_list": [{"model": "gpt-4", "api_key": os.getenv("OPENAI_API_KEY")}],
    },
)

user_proxy = UserProxyAgent(
    name="user",
    human_input_mode="NEVER",
    code_execution_config=False,
)

if __name__ == "__main__":
    user_proxy.initiate_chat(
        assistant,
        message="Search for quarterly reports and summarize them.",
    )
