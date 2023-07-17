import axios from "axios";

interface Cookie {
  name: string;
  value: string;
  expires?: Date;
  path?: string;
  sameSite?: "Lax" | "None" | "Strict";
  secure?: boolean;
}

class CookieJar {
  constructor(private jar = new Map<string, Cookie[]>()) {}

  setCookie(url: URL, cookieStr: string) {
    const key = url.origin.toLowerCase();
    if (!this.jar.has(key)) {
      this.jar.set(key, []);
    }

    const cookie = CookieJar.parse(cookieStr);
    this.jar.set(key, [...(this.jar.get(key)?.filter((c) => c.name !== cookie.name) || []), cookie]);
  }

  getCookies(url: URL): Cookie[] {
    const key = url.origin.toLowerCase();
    if (!this.jar.get(key)) {
      return [];
    }

    // Filter out expired cookies
    return this.jar.get(key)?.filter((cookie) => !cookie.expires || cookie.expires > new Date()) || [];
  }

  static parse(str: string): Cookie {
    if (typeof str !== "string") {
      throw new Error("argument str must be a string");
    }

    const parts = str.split(";").map((part) => part.trim());

    let cookie: Cookie;

    if (parts.length > 0) {
      const [name, value] = parts[0].split("=");
      if (!name || !value) {
        throw new Error("Invalid cookie");
      }

      cookie = {
        name,
        value,
      };
    } else {
      throw new Error("Invalid cookie");
    }

    parts.slice(1).forEach((part) => {
      const [name, value] = part.split("=");
      if (!name.trim()) {
        throw new Error("Invalid cookie");
      }

      const nameLow = name.toLowerCase();
      // eslint-disable-next-line quotes
      const val = value?.charAt(0) === "'" || value?.charAt(0) === '"' ? value?.slice(1, -1) : value;
      if (nameLow === "expires") {
        cookie.expires = new Date(val);
      }
      if (nameLow === "path") {
        cookie.path = val;
      }
      if (nameLow === "samesite") {
        if (val !== "Lax" && val !== "None" && val !== "Strict") {
          throw new Error("Invalid cookie SameSite value");
        }
        cookie.sameSite = val;
      }
      if (nameLow === "secure") {
        cookie.secure = true;
      }
    });

    return cookie;
  }
}
const jar = new CookieJar();

axios.interceptors.response.use((response) => {
  if (Array.isArray(response.headers["set-cookie"])) {
    response.headers["set-cookie"].forEach((c) => {
      jar.setCookie(new URL(response.config.url!), c);
    });
  }
  return response;
});
/* eslint-disable prefer-arrow-callback,func-names */
axios.interceptors.request.use(function (config) {
  const cookies = jar.getCookies(new URL(config.url!));

  if (cookies?.length > 0 && config.headers) {
    /* eslint-disable no-param-reassign */
    config.headers.cookie = cookies.map((cookie) => `${cookie.name}=${cookie.value}`).join("; ");
  }
  return config;
});
