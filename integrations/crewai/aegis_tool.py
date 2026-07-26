"""CrewAI integration with AEGIS sidecar.

Usage:
    pip install crewai requests
    python aegis_tool.py
"""

import os
import requests
from crewai import Agent, Task, Crew
from crewai.tools import BaseTool

SIDECAR_URL = os.getenv("AEGIS_SIDECAR_URL", "http://localhost:9000")


class AEGISGovernedTool(BaseTool):
    name: str = ""
    description: str = ""

    def _run(self, **kwargs) -> str:
        resp = requests.post(
            f"{SIDECAR_URL}/v1/check-tool",
            json={
                "tool": self.name,
                "target": kwargs.get("target", ""),
                "agent_id": os.getenv("AEGIS_AGENT_ID", "crewai-agent"),
            },
        )
        if resp.status_code == 403:
            return f"ERROR: Tool '{self.name}' blocked by AEGIS governance"
        return f"Tool '{self.name}' executed with {kwargs}"


search_tool = AEGISGovernedTool(name="search_documents", description="Search internal documents")
db_tool = AEGISGovernedTool(name="query_database", description="Query the database")

agent = Agent(
    role="Researcher",
    goal="Find information safely",
    backstory="Governed by AEGIS sidecar",
    tools=[search_tool, db_tool],
    allow_delegation=False,
    verbose=True,
)

task = Task(
    description="Search for quarterly reports and summarize findings",
    expected_output="A brief summary of quarterly reports",
    agent=agent,
)

crew = Crew(agents=[agent], tasks=[task])

if __name__ == "__main__":
    result = crew.kickoff()
    print(result)
