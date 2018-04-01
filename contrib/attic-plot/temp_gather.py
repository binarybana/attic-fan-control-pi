# Imports the Google Cloud client library
from google.cloud import datastore

# Instantiates a client
datastore_client = datastore.Client()

q = datastore_client.query(kind='TemperatureRecord')

data = list(q.fetch())
