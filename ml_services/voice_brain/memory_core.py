import chromadb
from sentence_transformers import SentenceTransformer
from datetime import datetime
import json

class MERLINMemory:
	"""MERLIN's memory system using vector embeddings"""

	def __init__(self, persist_dir="./merlin_memory"):
		
		print("Initializing MERLIN memory core...")

		# Create ChromaDB client with persistence
		self.client = chromadb.PersistentClient(path=persist_dir)

		# Create or get collection for MERLIN's memory
		self.conversations = self.client.get_or_create_collection(
			name = "conversations",
			metadata = {"description": "MERLIN conversation history"}
		)

		# load embedding model
		print(" Loading Sentence-Transformer model...")
		self.encoder = SentenceTransformer('all-MiniLM-L6-v2', device = 'cpu')

		print(" MERLIN memory core initialized.")
		print(f" Storage: {persist_dir} ")
		print(f" Conversation stored: {self.conversations.count()}")
	
	def store(self, text, context = None):
		"""Store a conversation or fact in memory"""

		# Generate embedding
		embedding = self.encoder.encode(text).tolist()

		# Create metadata
		metadata = {
			'timestamp': datetime.now().isoformat(),
			'type': 'conversation',
			'context': context or 'general'
		}

		doc_id = f"mem_{datetime.now().timestamp()}"

		# Store in ChromaDB
		self.conversations.add(
			documents = [text],
			embeddings = [embedding],
			metadatas = [metadata],
			ids = [doc_id]
		)

		print(f"Stored: {text[:60]}...")
		return doc_id


	def query(self, question, n_results = 3):
		"""Search memory for relevant conversations"""

		# Generate query embedding
		query_embedding = self.encoder.encode(question).tolist()

		# Search in ChromaDB
		results = self.conversations.query(
			query_embeddings = [query_embedding],
			n_results = n_results
		)

		# Format results
		memories = []
		if results['documents'] and results['documents'][0]:
			for i, doc in enumerate(results['documents'][0]):
				memory = {
					'text': doc,
					'metadata' : results['metadatas'][0][i],
					'distance' : results['distances'][0][i] if 'distances' in results else None 
				}
				memories.append(memory)
		return memories
	
	def get_recent(self, n = 5):
		"""Get most recent mems"""

		results = self.conversations.get()
		if not results['documents']:
			return []
		
		# Combine results with metadata
		memories = []
		for i, doc in enumerate(results['documents']):
			memories.append({
				'text' : doc,
				'timestamp' : results['metadatas'][i]['timestamp']
			})

		# Sort by most recent timestamp
		memories.sort(key=lambda x: x['timestamp'], reverse = True)
		return memories[:n]
	
	def stats(self):
		"""Get memory stats"""
		count = self.conversations.count()
		recent = self.get_recent(1)
		return {
			'total_memories' : count,
			'latest_memory' : recent[0] if recent else None
		}
