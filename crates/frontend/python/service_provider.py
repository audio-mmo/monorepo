from protos.frontend_pb2 import ServiceRequestBatch

from services import speak, shutdown


class ServiceProvider:
    def __init__(self, client):
        self.client = client

    def tick(self):
        encoded_requests = self.client.dequeue_service_requests()
        requests = ServiceRequestBatch.FromString(encoded_requests)
        for req in requests.requests:
            self.handle_request(req)

    def handle_request(self, req):
        field = req.WhichOneof("service")
        if field == "speak":
            speak(req.speak.text, req.speak.interrupt)
        elif field == "shutdown":
            shutdown()
