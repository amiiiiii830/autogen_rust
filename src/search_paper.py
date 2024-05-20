import urllib.request
import urllib.parse
from xml.etree import ElementTree

def debug_arxiv_search():
    # Simplify the query to check basic functionality
    url = "http://export.arxiv.org/api/query?search_query=all:GPT&start=0&max_results=10"
    try:
        with urllib.request.urlopen(url) as response:
            result = response.read()
            return result
    except Exception as e:
        print(f"API call failed: {e}")
        return None

# Test the debugging API call
debug_result = debug_arxiv_search()
if debug_result:
    print(debug_result.decode('utf-8'))
else:
    print("No results or failed to fetch data.")