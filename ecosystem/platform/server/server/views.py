from django.views.generic import TemplateView

# Serves the build/index.html from the client-side app.
client_app = TemplateView.as_view(template_name='index.html')
