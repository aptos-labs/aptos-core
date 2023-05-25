# A wrapper around fetching time information

from datetime import datetime, timezone


class Time:
    def epoch(self) -> str:
        return self.now().strftime("%s")

    def now(self) -> datetime:
        raise NotImplementedError()


class SystemTime(Time):
    def now(self) -> datetime:
        return datetime.now(timezone.utc)


class FakeTime(Time):
    _now: int = 1659078000

    def now(self) -> datetime:
        return datetime.fromtimestamp(self._now, timezone.utc)

    def epoch(self) -> str:
        return str(self._now)
