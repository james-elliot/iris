# Retrouve à partir d'une adresse le numéro de la maille IRIS de l'INSEE

La v1 utilisait les données `iris.geojson` disponibles
[ici](https://data.opendatasoft.com/explore/dataset/iris@datailedefrance/export/)
et les adresses de la base de données gouvernementales disponibles
[là](https://adresse.data.gouv.fr/data/ban/adresses/latest/csv/adresses-france.csv.gz).
Les fichiers doivent être placés dans le répertoire contenant le programme.
***La v1 est désormais obsolète et est remplacée par la v2.***

Le fichier `iris.geojson` distribué sur le site opendatasoft était instable.
***La v2 ne l'utilise plus.***
Elle utilise directement les données au format shapefile distribuées
par l'IGN.
***Il est donc désormais inutile de récupérer le fichier iris.geojson.***
A la place il faut utiliser la procédure suivante: 
- récupérer l'archive des contours IRIS disponibles
[ici](https://geoservices.ign.fr/contoursiris).
Pour l'année 2022 par exemple, il faut récupérer l'archive
`CONTOURS-IRIS_2-1__SHP__FRA_2022-01-01.7z`
- trouver dans l'arborescence de l'archive le répertoire 
`CONTOURS-IRIS_2-1_SHP_LAMB93_FXX-2022` (si l'on travaille sur 2022, à adapter évidemment pour une autre année)
- extraire ce répertoire dans le répertoire qui contient le programme et le renommer simplement en 
`CONTOURS`. 

De cette façon, il est possible d'utiliser les données de maille pour n'importe quelle année.

Les données d'adresses sont en coordonnées WGS84 alors que les données de la maille IRIS sont en format Lambert93.
La conversion est faite par le programme en se basant sur la crate Rust `lambert`. Nous n'avons pas noté de différence 
avec le fichier au format geojson.

Le programme a été écrit pour travailler avec en entrée un fichier au format CSV contenant 4 colonnes:
un identifiant numérique `N_PATIENT`,
l'adresse de la rue `N_PST`,
le code postal `N_CP`
et la nom de la ville `PST_VILLE`.

Les données sont ensuite normalisées puis l'algorithme utilise des techniques de fuzzy_match pour tenter de trouver 
l'adresse s'approchant le plus de celle fournie. 

Si l'adresse exacte est trouvée, le résultat est écrit dans le fichier `ok.csv`
dont les 4 premières colonnes reprennent les informations du fichier d'origine. Les colonnes suivantes sont
`n_adresse` qui est l'adresse de la rue normalisée trouvée par le programme,
`n_cp` qui est le code postal trouvé,
`n_ville` qui est le nom de la ville trouvée,
`iris` qui est le numéro de la maille IRIS,
`s_ville` qui est l'indice de confiance pour la ville entre 0 et 1.0 (dans ce cas 1.0) 
`s_adresse` qui est l'indice de confiance pour l'adresse (ici 1.0 également).

Si une adresse proche est trouvée les résultats sont écrits dans le fichier `sok.csv` suivant le même
format que celui de `ok.csv` mais avec des indices de confiance inférieurs à 1.0.

Si aucune adresse proche n'est trouvée, les données sont reproduites à l'identique dans le fichier `nok.csv`.
