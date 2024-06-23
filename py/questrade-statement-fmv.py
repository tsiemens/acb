#!/usr/bin/env python3
import ensure_in_venv

import argparse
import io
import re
import sys
from abc import ABC, abstractmethod
from csv import QUOTE_NONNUMERIC, writer
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from enum import Enum
from typing import Optional

import PyPDF2


class ParseError(Exception):
    pass


class SecOwnedState(Enum):
    start = "start"
    hdrAllocation = "hdrAllocation"
    hdrMarketValue = "hdrMarketValue"
    div = "div"
    security = "security"
    allocation = "allocation"
    marketValue = "marketValue"
    done = "done"


@dataclass(frozen=True)
class FMV:
    security: str
    allocation: Decimal
    fmv: Decimal


@dataclass
class ParseState:
    security: list[str]
    allocation: Optional[Decimal] = None
    fmv: Optional[Decimal] = None

    def to_fmv(self) -> FMV:
        assert self.security
        assert self.allocation is not None
        assert self.fmv is not None

        sec = " ".join(self.security)
        m = re.match(r".*\((?P<ticker>.*?)\)\s*$", sec)
        assert m is not None
        return FMV(
            security=m.group("ticker"),
            allocation=self.allocation,
            fmv=self.fmv,
        )


def newParseState() -> ParseState:
    return ParseState([])


class Matcher(ABC):
    @abstractmethod
    def match_line(self, v: str) -> bool:
        ...

    def to_decimal(self, v: str) -> Decimal:
        raise ValueError(
            f"{self.__class__.__name__}({v}) is not convertible to Decimal"
        )


class NullMatcher(Matcher):
    def match_line(self, v: str) -> bool:
        _ = v
        raise NotImplementedError


class StrMatcher(Matcher):
    def __init__(self, crit: str) -> None:
        self.crit = crit

    def match_line(self, v: str) -> bool:
        return v == self.crit


class ReMatcher(Matcher):
    def __init__(self, pat: str, flags: int = 0) -> None:
        self.crit = re.compile(pat, flags=flags)

    def match_line(self, v: str) -> bool:
        return self.crit.match(v) is not None


class NegReMatcher(ReMatcher):
    def match_line(self, v: str) -> bool:
        return not super().match_line(v)


class DecMatcher(ReMatcher):
    def __init__(self) -> None:
        super().__init__(r"-?\s*[., \d]+")

    def to_decimal(self, v: str) -> Decimal:
        neg = -1 if v[0] == "-" else 1
        x = re.sub(r"[, ]", "", v)
        try:
            return neg * Decimal(x)
        except InvalidOperation as e:
            raise ParseError(f"cannot convert {v!r} to Decimal") from e


SEC_PAT = r"^[A-Z].*"

STATES: dict[SecOwnedState, list[tuple[Matcher, SecOwnedState]]] = {
    SecOwnedState.start: [
        (StrMatcher("ALLOCATION"), SecOwnedState.hdrAllocation),
    ],
    SecOwnedState.hdrAllocation: [
        (StrMatcher("MARKET VALUE"), SecOwnedState.hdrMarketValue),
    ],
    SecOwnedState.hdrMarketValue: [
        (ReMatcher(SEC_PAT), SecOwnedState.security),
    ],
    SecOwnedState.div: [
        (ReMatcher(SEC_PAT), SecOwnedState.security),
    ],
    SecOwnedState.security: [
        (DecMatcher(), SecOwnedState.allocation),
    ],
    SecOwnedState.allocation: [
        (DecMatcher(), SecOwnedState.marketValue),
    ],
    SecOwnedState.marketValue: [
        (StrMatcher("100.0"), SecOwnedState.done),
        (NegReMatcher(SEC_PAT), SecOwnedState.div),
        (ReMatcher(SEC_PAT), SecOwnedState.security),
    ],
}


def parse_pdf(fp: io.BufferedReader) -> tuple[list[FMV], str]:
    reader = PyPDF2.PdfReader(fp)

    fmvs: list[FMV] = []
    month = None
    for page in reader.pages:
        text = page.extract_text()
        if "current month" in text.lower():
            if m := re.search(
                r"\bCurrent month:\s+(?P<month>\S+ \d+, \d+)\n", text, re.IGNORECASE
            ):
                month = m.group("month")
        if "Securities Owned\nCombined in (CAD)" not in text:
            continue
        state = SecOwnedState.start
        conv: Matcher = NullMatcher()
        accum = newParseState()
        for line in (l.strip() for l in text.splitlines()):
            for matcher in STATES[state]:
                if matcher[0].match_line(line):
                    conv = matcher[0]
                    state = matcher[1]
                    break

            if state == SecOwnedState.done:
                break
            elif state == SecOwnedState.security:
                accum.security.append(line)
            elif state == SecOwnedState.allocation:
                assert (
                    accum.allocation is None
                ), f"already have allocation {accum.allocation}"
                accum.allocation = conv.to_decimal(line)
            elif state == SecOwnedState.marketValue:
                assert accum.fmv is None
                accum.fmv = conv.to_decimal(line)
                fmvs.append(accum.to_fmv())
                accum = newParseState()

        assert (
            state == SecOwnedState.done
        ), f"expected to terminal in 'done', but in {state}"

    assert month is not None
    return fmvs, month


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", metavar="FILES", nargs="+")
    args = ap.parse_args()

    securities: set[str] = set()
    values: list[tuple[str, list[FMV]]] = []

    for filename in sorted(args.files):
        with open(filename, "rb") as fp:
            fmvs, month = parse_pdf(fp)
            securities |= {f.security for f in fmvs}
            values.append((month, fmvs))

    header = ["Month", "Total FMV (CAD)"] + sorted(securities)
    w = writer(sys.stdout, delimiter=",", quoting=QUOTE_NONNUMERIC)
    w.writerow(header)

    for month, fmvs in values:
        total = sum(f.fmv for f in fmvs)
        secmap = {f.security: f for f in fmvs}
        row = [month, total]
        for sec in sorted(securities):
            if sec not in secmap:
                row.append("-")
                continue
            fmv = secmap[sec]
            row.append(fmv.fmv)

        w.writerow(row)


if __name__ == "__main__":
    main()
