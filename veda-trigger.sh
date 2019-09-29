#!/bin/bash
username="admin"
password="a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3&1569718747374"

contentText="Прошу обратить внимание на неподобающее поведение сантехника"

partRandom=$RANDOM
echo "partRandom = $partRandom"

individual='{"@":"hack:incomingRequest'$partRandom'","rdf:type":[{"type":"Uri","data":"hack:Request"}], "v-s:content":[{"type":"String", "data":"'$contentText'"}]}'
echo "individual = $individual"

auth_request="curl -s -X GET http://localhost:80/authenticate?login=$username&password=$password"
#echo "auth_request = $auth_request"

auth_result=$($auth_request)
#echo "auth_result = $auth_result"

ticket=$(echo $auth_result | grep -oP '(?<="id":")[\w\d-]+(?=")')
echo "ticket = $ticket"

payload='{"ticket":"'$ticket'","individual":'$individual',"prepare_events":true,"event_id":"","transaction_id":""}'
echo "payload = $payload"

put_request="curl -s -X PUT -H 'Content-Type: application/json' http://localhost:$port/put_individual -d '$payload'"
#echo "put_request = $put_request"

put_result=$(eval $put_request)
echo "put_result = $put_result"
