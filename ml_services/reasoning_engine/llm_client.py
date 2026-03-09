import requests
import json
from typing import List, Dict, Optional

class LLMClient:
    def __init__(self, model="gemma3:1b", base_url="http://localhost:11434"):
        """Init local LLM client: Args: model name, base url"""
        self.model = model
        self.base_url = base_url
        self.generate_url = f"{base_url}/api/generate"

        print(f"LLM Client initialized: {model} (local)")
        
    def generate_response(
            self, 
            user_input: str, 
            memory_context: Optional[List[Dict]] = None, 
            system_prompt: Optional[str] = None
    ) -> str:
        """Generate response from LLM with memory context"""
        """Args: user input, memory context, system prompt"""
        # default prompt, change later
        if system_prompt is None: 
            system_prompt = "You are MERLIN, an intelligent assistant with memory. " \
            "You are concise, loyal, and proactive. You remember past conversations." \
            "Keep responses brief (1-2 sentences)"
        prompt = f"{system_prompt}\n\n"

        # Add memory context
        if memory_context:
            prompt += "Recent relevant context:\n"
            for mem in memory_context[:3]: # top 3 memories
                prompt += f"- User: {mem['user_input']}\n"
                prompt += f" You:{mem['merlin_response']}\n"
            prompt += "\n"
        
        # current user_input
        prompt += f"User: {user_input}\nMerlin:"

        # Call LLM API
        try:
            response = requests.post(
                self.generate_url,
                json={
                    "model": self.model,
                    "prompt": prompt,
                    "stream": False,
                    "options": {
                        "temperature": 0.7,
                        "num_predict": 50, # Max tokens (short responses for now)
                        "top_k": 40,
                        "top_p": 0.9
                    }
                },
                timeout = 30
            )

            if response.status_code == 200:
                result = response.json()
                return result["response"].strip()
            else:
                raise Exception(f"Ollama API error: {response.status_code}")
        except requests.exceptions.RequestException as e:
            print(f"LLM request failed: {e}")
            return "I'm having trouble thinking right now..."
        
# Test
if __name__ == "__main__":
    print ("Testing LLM Client...")
    llm = LLMClient(model="gemma3:1b")
    response = llm.generate_response("What's your name?")
    print(f"\n MERLIN: {response}")

    # Test memories
    test_context = [
        {
            "user_input": "My name is Alex",
            "merlin_response": "Nice to meet you preBirth Alex!"
        }
    ]

    response = llm.generate_response("What is my name?", memory_context=test_context)
    print(f"\n MERLIN with memory: {response}")

