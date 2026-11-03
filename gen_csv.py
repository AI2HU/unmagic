#!/usr/bin/env python3
"""Generate a fake CSV (default 1,000,000 rows) for the parser benchmark.

Usage: python3 gen_csv.py [rows] [output]
"""
import random
import sys

rows = int(sys.argv[1]) if len(sys.argv) > 1 else 1_000_000
out = sys.argv[2] if len(sys.argv) > 2 else "data.csv"

FIRST = ["Alice", "Bob", "Chloe", "David", "Emma", "Felix", "Grace", "Hugo",
         "Iris", "Jules", "Lea", "Marc", "Nina", "Oscar", "Paula", "Quentin"]
LAST = ["Martin", "Bernard", "Dubois", "Thomas", "Robert", "Richard", "Petit",
        "Durand", "Leroy", "Moreau", "Simon", "Laurent", "Lefebvre", "Michel"]
CITY = ["Paris", "Lyon", "Marseille", "Toulouse", "Nice", "Nantes",
        "Strasbourg", "Montpellier", "Bordeaux", "Lille", "Rennes", "Reims"]
DOMAIN = ["example.com", "mail.test", "corp.example", "demo.org"]

rng = random.Random(42)  # fixed seed: reproducible file

with open(out, "w", newline="\n") as f:
    f.write("id,name,email,age,salary,city,active,score\n")
    for i in range(1, rows + 1):
        first, last = rng.choice(FIRST), rng.choice(LAST)
        email = f"{first}.{last}{i}@{rng.choice(DOMAIN)}".lower()
        f.write(
            f"{i},{first} {last},{email},{rng.randint(18, 80)},"
            f"{rng.randint(20000, 150000)}.{rng.randint(0, 99):02d},"
            f"{rng.choice(CITY)},{rng.choice(['true', 'false'])},"
            f"{rng.randint(0, 100)}.{rng.randint(0, 99):02d}\n"
        )

print(f"wrote {rows} rows to {out}")
