import sys

# COMMON
if len(sys.argv) > 3 and sys.argv[3] == 'local':
    EMAIL = "infrastructure@alxshelepenok.local"
    BASE_HOSTNAME = "infrastructure.alxshelepenok.local"
else:
    EMAIL = "infrastructure@alxshelepenok.com"
    BASE_HOSTNAME = "infrastructure.alxshelepenok.com"

# NEXUS
NEXUS_PORT = 8081
NEXUS_HOST = 'nexus.{0}'.format(BASE_HOSTNAME)

# NEXUS DOCKER GROUP
NEXUS_DOCKER_GROUP_PORT = 8000
NEXUS_DOCKER_GROUP_REMOTE_PORT = 9000
NEXUS_DOCKER_GROUP_REMOTE_HOST = "nexus"
NEXUS_DOCKER_GROUP_HOST = 'docker-group.nexus.{0}'.format(BASE_HOSTNAME)

# NEXUS DOCKER HOSTED
NEXUS_DOCKER_HOSTED_PORT = 8000
NEXUS_DOCKER_HOSTED_REMOTE_PORT = 9001
NEXUS_DOCKER_HOSTED_REMOTE_HOST = "nexus"
NEXUS_DOCKER_HOSTED_HOST = "docker-hosted.nexus.{0}".format(BASE_HOSTNAME)

# NEXUS DOCKER PROXY
NEXUS_DOCKER_PROXY_PORT = 8000
NEXUS_DOCKER_PROXY_REMOTE_PORT = 9002
NEXUS_DOCKER_PROXY_REMOTE_HOST = "nexus"
NEXUS_DOCKER_PROXY_HOST = "docker-proxy.nexus.{0}".format(BASE_HOSTNAME)

# GRAFANA
GRAFANA_PORT = 3000
GRAFANA_HOST = "grafana.{0}".format(BASE_HOSTNAME)


def main():
    if len(sys.argv) < 2:
        print(u'Usage: python create-environment.py [name] [dest]')
        return

    env_values = {}
    env_groups = {}

    if sys.argv[1] == 'infrastructure':
        env_groups.update({
            'NEXUS': [
                'NEXUS_PORT',
                'NEXUS_HOST',
                'NEXUS_EMAIL'
            ],
            'NEXUS DOCKER GROUP': [
                'NEXUS_DOCKER_GROUP_PORT',
                'NEXUS_DOCKER_GROUP_REMOTE_PORT',
                'NEXUS_DOCKER_GROUP_REMOTE_HOST',
                'NEXUS_DOCKER_GROUP_HOST',
                'NEXUS_DOCKER_GROUP_EMAIL',
            ],
            'NEXUS DOCKER PROXY': [
                'NEXUS_DOCKER_PROXY_PORT',
                'NEXUS_DOCKER_PROXY_REMOTE_PORT',
                'NEXUS_DOCKER_PROXY_REMOTE_HOST',
                'NEXUS_DOCKER_PROXY_HOST',
                'NEXUS_DOCKER_PROXY_EMAIL',
            ],
            'NEXUS DOCKER HOSTED': [
                'NEXUS_DOCKER_HOSTED_PORT',
                'NEXUS_DOCKER_HOSTED_REMOTE_PORT',
                'NEXUS_DOCKER_HOSTED_REMOTE_HOST',
                'NEXUS_DOCKER_HOSTED_HOST',
                'NEXUS_DOCKER_HOSTED_EMAIL',
            ],
            'GRAFANA': [
                'GRAFANA_PORT',
                'GRAFANA_HOST',
                'GRAFANA_EMAIL',
            ],
        })
        env_values.update({
            'NEXUS_PORT': NEXUS_PORT,
            'NEXUS_HOST': NEXUS_HOST,
            'NEXUS_EMAIL': EMAIL,
            'NEXUS_DOCKER_GROUP_PORT': NEXUS_DOCKER_GROUP_PORT,
            'NEXUS_DOCKER_GROUP_REMOTE_PORT': NEXUS_DOCKER_GROUP_REMOTE_PORT,
            'NEXUS_DOCKER_GROUP_REMOTE_HOST': NEXUS_DOCKER_GROUP_REMOTE_HOST,
            'NEXUS_DOCKER_GROUP_HOST': NEXUS_DOCKER_GROUP_HOST,
            'NEXUS_DOCKER_GROUP_EMAIL': EMAIL,
            'NEXUS_DOCKER_PROXY_PORT': NEXUS_DOCKER_PROXY_PORT,
            'NEXUS_DOCKER_PROXY_REMOTE_PORT': NEXUS_DOCKER_PROXY_REMOTE_PORT,
            'NEXUS_DOCKER_PROXY_REMOTE_HOST': NEXUS_DOCKER_PROXY_REMOTE_HOST,
            'NEXUS_DOCKER_PROXY_HOST': NEXUS_DOCKER_PROXY_HOST,
            'NEXUS_DOCKER_PROXY_EMAIL': EMAIL,
            'NEXUS_DOCKER_HOSTED_PORT': NEXUS_DOCKER_HOSTED_PORT,
            'NEXUS_DOCKER_HOSTED_REMOTE_PORT': NEXUS_DOCKER_HOSTED_REMOTE_PORT,
            'NEXUS_DOCKER_HOSTED_REMOTE_HOST': NEXUS_DOCKER_HOSTED_REMOTE_HOST,
            'NEXUS_DOCKER_HOSTED_HOST': NEXUS_DOCKER_HOSTED_HOST,
            'NEXUS_DOCKER_HOSTED_EMAIL': EMAIL,
            'GRAFANA_PORT': GRAFANA_PORT,
            'GRAFANA_HOST': GRAFANA_HOST,
            'GRAFANA_EMAIL': EMAIL,
        })

    with open(sys.argv[2], "w") as outfile:
        env_groups_items = env_groups.items()
        for idx, [group, env_vars] in enumerate(env_groups_items):
            outfile.write(f'# {group}\n')
            for var in env_vars:
                outfile.write(f'{var}={env_values[var]}\n')

            if idx != len(env_groups) - 1:
                outfile.write('\n')


if __name__ == '__main__':
    main()
