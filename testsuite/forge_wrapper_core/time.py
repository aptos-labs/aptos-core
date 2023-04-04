from datetime import datetime, timezone


class Time:
    def epoch(self) -> str:
        return self.now().strftime("%s")

    def now(self) -> datetime:
        raise NotImplementedError()


class SystemTime(Time):
    def now(self) -> datetime:
        return datetime.now(timezone.utc)
