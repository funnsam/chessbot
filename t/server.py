import http.server
import time

class rh(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        while True:
            self.wfile.write(b'heartbeat\n')
            time.sleep(1)

s = http.server.HTTPServer(("", 8000), rh)
s.serve_forever()
