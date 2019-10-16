vcl 4.0;

# https://docs.varnish-software.com/tutorials/hit-miss-logging/

sub vcl_recv {
  unset req.http.x-cache;
  unset req.http.Cookie;
}

sub vcl_hit {
  set req.http.x-cache = "hit";
}

sub vcl_miss {
  set req.http.x-cache = "miss";
}

sub vcl_pass {
  set req.http.x-cache = "pass";
}

sub vcl_pipe {
  set req.http.x-cache = "pipe uncacheable";
}

sub vcl_synth {
  set resp.http.x-cache = "synth synth";
}

sub vcl_deliver {
  if (obj.uncacheable) {
    set req.http.x-cache = req.http.x-cache + " uncacheable" ;
  } else {
    set req.http.x-cache = req.http.x-cache + " cached" ;
  }

  set resp.http.x-cache = req.http.x-cache;
}

backend default {
  .host = "web";
  .port = "3000";
}
