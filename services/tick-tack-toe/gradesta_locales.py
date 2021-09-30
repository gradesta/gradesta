from dataclasses import dataclass


@dataclass
class Localizer:
    service_name: str
    locale: str = "en"

    def fmt(self, string: str, *args, **kwargs):
        return string.format(*args, **kwargs)

    def service_names():
        return {"en": self.service_name}
