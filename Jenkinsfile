@Library('infrastructure-jenkins-shared-library@master')_

G_gitcred = "LaninSSHgit"
G_promoted_branch = 'origin/master'
G_buildstatus = 'NotSet'
G_teststatus = 'NotSet'

pipeline {
    parameters {
        booleanParam (
            defaultValue: false,
            description: 'Promote image built to be used as latest',
            name : 'FORCE_PROMOTE_LATEST'
        )
    }
    agent {
        node {label 'master'}
    }
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    stages {
        stage ('Build') {
            steps {
                script {
                    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    sshagent (credentials: [G_gitcred]) {
                        withEnv(['DOCKER_BUILDKIT=1']) {
                            app_image = docker.build(
                                "${G_dockerimage}",
                                '--label "git-commit=${GIT_COMMIT}" --ssh default .'
                            )
                        }
                    }
                }
            }
        }
	stage('Test') { 
            steps {
                script {
		    G_dockerimage = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    docker.image(G_dockerimage).inside("-u root") {
				sh 'sh prerequesites.sh'
				sh 'tvm_linker --version'
		    }
                }
            }
        }
        stage('Push docker-image') {
            steps {
                script {
                    docker.withRegistry('', 'dockerhubLanin') {
                        app_image.push()
                    }
                }
            }
            post {
                success {
                    script{
                        G_buildstatus = "success"
                    }
                }
                failure {script{G_buildstatus = "failure"}}
            }
        } 
        stage ('Tag as latest') {
            when {
                expression {
                    GIT_BRANCH = 'origin/' + sh(returnStdout: true, script: 'git rev-parse --abbrev-ref HEAD').trim()
                    return GIT_BRANCH == G_promoted_branch || params.FORCE_PROMOTE_LATEST
                }
            }
            steps {
                script {
                    docker.withRegistry('', 'dockerhubLanin') {
                        docker.image("${G_dockerimage}").push('latest')
                    }
                }
            }
        }
    }
    post {
        always {
            notifyTeam(
                buildstatus: G_buildstatus
            ) 
        }
    }
}
